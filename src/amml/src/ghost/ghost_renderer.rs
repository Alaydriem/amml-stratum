use std::ffi::c_void;
use std::ptr::NonNull;
use std::time::Instant;

use amml_game::{ClientInstance, GameEntries, RawObject, RenderCamera};
use stratum_core::structs::ghost::{GhostJob, GhostJobMode, SlotBuilt};

use crate::game::{OwnedMesh, ScreenContext, Tessellator, TexturePtrLayout, TextureVariant};
use crate::ghost::{BlockResolver, GhostGroupBuffer, GhostTessellator};
use crate::services::{GhostTake, SharedGhost};
use crate::types::{GhostBuilt, GhostCalls, GhostWorld, GroupStep, PlayerSpot};

/// The ghost on the render thread: each frame it takes the frame core published, frees and builds the mesh slots it
/// lists, draws the listed slots that are in view see-through over the world, farthest first, and reports what it
/// built and how many it left out of view. What to build and draw is core's; the renderer holds only the slot table
/// and its per-slot scratch, sized once to core's slot capacity, and the palette's block lookups.
///
/// Bound: the slot table and every per-slot vector hold `slot_capacity` entries, which changes only with the config;
/// the block lookups hold one entry per palette id.
pub struct GhostRenderer {
    calls: GhostCalls,
    // the world the slots, the blocks and the tessellator belong to; null outside a world
    region: *mut c_void,
    usable: bool,
    blocks: BlockResolver,
    tessellator: GhostTessellator,
    slots: Vec<Option<OwnedMesh>>,
    // the mini ghost group being built and each slot's last finished group
    group: GhostGroupBuffer,
    // the generation the group buffer belongs to
    generation: u64,
    // one entry per job of a frame, at most one per slot
    built: Vec<SlotBuilt>,
    // per slot: the built box in cells, and its squared distance this frame
    sizes: Vec<[u32; 3]>,
    distance: Vec<f32>,
    // this frame's draw indices in view, farthest first
    order: Vec<u32>,
    culled: u32,
    meshes_drawn: u32,
    // set on a join, cleared once a report has carried it
    world_reset: bool,
    // core's world epoch the slots, the blocks and the tessellator belong to
    world_epoch: u64,
    // this frame resolved a palette; its fallback counts go in the report
    palette_resolved: bool,
    // this frame's build time, the sum of its jobs, and its built and skipped cells
    build_us: u64,
    cells_built: u32,
    skipped_cells: u32,
}

// SAFETY: the raw game pointers and the meshes are used on the render thread only, under the runtime's lock; another
// thread only moves the renderer, it never calls through it
#[allow(unsafe_code)]
unsafe impl Send for GhostRenderer {}

impl GhostRenderer {
    /// What core counts a built mesh's GPU memory at: the measured vertex size on the GPU.
    pub const GPU_BYTES_PER_VERTEX: u64 = 20;
    /// A group mesh's own index buffer holds 32-bit indices.
    pub const INDEX_BYTES: u64 = 4;
    const ALPHA_MAX: f32 = 255.0;
    const CORNERS: usize = 8;

    pub fn new(calls: &GhostCalls) -> Self {
        Self {
            calls: *calls,
            region: std::ptr::null_mut(),
            usable: false,
            blocks: BlockResolver::new(calls),
            tessellator: GhostTessellator::new(calls),
            slots: Vec::new(),
            group: GhostGroupBuffer::new(calls),
            generation: 0,
            built: Vec::new(),
            sizes: Vec::new(),
            distance: Vec::new(),
            order: Vec::new(),
            culled: 0,
            meshes_drawn: 0,
            world_reset: false,
            world_epoch: 0,
            palette_resolved: false,
            build_us: 0,
            cells_built: 0,
            skipped_cells: 0,
        }
    }

    /// After `LevelRenderer::renderLevel`'s own drawing: reads the world and the camera from the game, then runs the
    /// frame. `client` is the client the tick saw last.
    ///
    /// # Safety
    /// `renderer`, `screen` and `frame` must be the live arguments of the running `renderLevel` call, and `client`
    /// null or the live client of the game.
    #[allow(unsafe_code)]
    pub unsafe fn after_render_level(
        &mut self,
        entries: &GameEntries,
        client: Option<NonNull<c_void>>,
        renderer: *mut c_void,
        screen: *mut c_void,
        frame: *const c_void,
        ghost: &SharedGhost,
    ) {
        // SAFETY: the ScreenContext of the running render call
        let Some(screen) = (unsafe { ScreenContext::from_raw(screen, &self.calls) }) else { return };
        // SAFETY: forwarded to the caller
        let world = unsafe { self.world_of(entries, client, renderer) };
        // SAFETY: forwarded to the caller
        let camera = unsafe { self.camera_of(entries, frame) }.unwrap_or([0.0; 3]);
        self.run_frame(world.as_ref(), &screen, ghost, camera);
    }

    /// The block of palette id `id` in the joined world, for easy place's creative pick; null outside a world, while
    /// the renderer is not usable, or for an id the palette does not resolve.
    pub fn palette_block(&self, id: u16) -> *const c_void {
        if self.region.is_null() || !self.usable {
            return std::ptr::null();
        }
        self.blocks.block(id)
    }

    /// One frame: take, free, build, cull and sort, draw, report, in the order the C++ renderer did them. `world` is
    /// None outside a world or with no local player; `camera` is the frame's camera position.
    #[allow(unsafe_code)]
    pub fn run_frame(
        &mut self,
        world: Option<&GhostWorld>,
        screen: &ScreenContext,
        ghost: &SharedGhost,
        camera: [f32; 3],
    ) {
        let Some(world) = world.filter(|world| !world.region.is_null()) else {
            if !self.region.is_null() {
                self.leave();
            }
            return;
        };
        let Some(tessellator) = screen.tessellator() else { return };
        // SAFETY: the ScreenContext's own tessellator, live for the frame
        let Some(mut tessellator) = (unsafe { Tessellator::from_raw(tessellator, &self.calls) }) else { return };
        let Some(take) = ghost.take() else { return };
        // core's epoch is the one trigger of a rejoin: it moves once per world, also when the new region sits at the
        // old address, and core has already started the new generation then. While core is between two worlds the
        // renderer holds nothing, so a new region seen early does not join. A region that changes under an unchanged
        // epoch is the safety net: the join then asks core for a new generation
        let epoch_moved = take.world_epoch() != self.world_epoch;
        self.world_epoch = take.world_epoch();
        if !take.in_world() {
            if !self.region.is_null() {
                self.leave();
            }
        } else if epoch_moved || world.region != self.region {
            if !self.region.is_null() {
                self.leave();
            }
            self.join(world, &mut tessellator, !epoch_moved);
        }
        self.apply(&take, &mut tessellator);
        let draw_us = self.draw(&take, screen, world, camera);
        drop(take);
        let unknown = self.blocks.take_unknown();
        let default_state = self.blocks.take_default_state();
        let world_reset = self.world_reset;
        let Self { built, palette_resolved, build_us, meshes_drawn, cells_built, skipped_cells, culled, .. } = self;
        let sent = ghost.report(world_reset, |report| {
            report.built.clear();
            report.built.extend_from_slice(built);
            report.unknown_blocks = unknown;
            report.default_state_blocks = default_state;
            report.palette_resolved = *palette_resolved;
            report.build_us = *build_us;
            report.draw_us_sum = draw_us;
            report.draw_frames = 1;
            report.draw_us_max = draw_us;
            report.meshes_drawn = *meshes_drawn;
            report.cells_built = *cells_built;
            report.skipped_cells = *skipped_cells;
            report.culled = *culled;
            report.world_reset = world_reset;
        });
        if sent.is_some() {
            self.world_reset = false;
        }
    }

    /// Drops every mesh and lookup, as a world exit does; the next frame in a world joins again. Render thread.
    pub fn reset(&mut self) {
        self.leave();
        self.world_epoch = 0;
    }

    #[allow(unsafe_code)]
    fn join(&mut self, world: &GhostWorld, tessellator: &mut Tessellator, ask_reset: bool) {
        self.region = world.region;
        // the meshes and block lookups are gone; unless core started a new generation for this join already, it
        // starts one with the palette and a clear
        self.world_reset = self.world_reset || ask_reset;
        let blocks_ok = self.blocks.join();
        // SAFETY: the frame's live client region and game tessellator
        self.usable =
            blocks_ok && unsafe { self.tessellator.join(world.region, &self.blocks, tessellator, world.feet) };
        curia::info!("ghost renderer joined the world", { usable: self.usable, blend_material: !world.blend.is_null() });
    }

    // a Block pointer and the tessellator's region belong to the world that was left, so both go with the meshes
    fn leave(&mut self) {
        self.slots.iter_mut().for_each(|slot| *slot = None);
        self.group.clear();
        self.tessellator.leave();
        self.blocks.leave();
        self.region = std::ptr::null_mut();
        self.usable = false;
    }

    // in core's order: free every slot, then the frees, then the palette, then the jobs
    fn apply(&mut self, take: &GhostTake, tessellator: &mut Tessellator) {
        let capacity = take.slot_capacity() as usize;
        if capacity != self.slots.len() {
            // sized once to core's slot capacity, which changes only with the config
            self.slots.clear();
            self.slots.resize_with(capacity, || None);
            self.built.clear();
            self.built.reserve(capacity);
            self.sizes = vec![[0; 3]; capacity];
            self.distance = vec![0.0; capacity];
            self.order = vec![0; capacity];
            self.group.resize(capacity);
        }
        // a group build of an earlier generation is never finished
        if take.clear_all() || take.generation() != self.generation {
            self.group.clear();
        }
        self.generation = take.generation();
        if take.clear_all() {
            self.slots.iter_mut().for_each(|slot| *slot = None);
        }
        for free in take.frees() {
            if let Some(slot) = self.slots.get_mut(*free as usize) {
                *slot = None;
                self.group.free(*free);
            }
        }
        self.palette_resolved = false;
        if let Some(palette) = take.palette() {
            self.palette_resolved = true;
            let start = Instant::now();
            self.blocks.resolve(palette, take.rotation());
            curia::info!("ghost palette resolved", { us: Self::micros(start) });
        }
        self.build(take, tessellator);
    }

    fn build(&mut self, take: &GhostTake, tessellator: &mut Tessellator) {
        self.built.clear();
        self.build_us = 0;
        self.cells_built = 0;
        self.skipped_cells = 0;
        let alpha = (take.opacity().clamp(0.0, 1.0) * Self::ALPHA_MAX).round() as u8;
        for (generation, job) in take.jobs() {
            // a job of another generation belongs to a build core has already replaced
            if *generation != take.generation()
                || job.slot as usize >= self.slots.len()
                || self.built.len() >= self.slots.len()
            {
                continue;
            }
            let start = Instant::now();
            let idle = if job.mode == GhostJobMode::Replace {
                self.build_replace(job, *generation, tessellator, alpha)
            } else {
                self.build_group(job, *generation, tessellator, alpha)
            };
            // the game is tessellating: this job and the rest go unanswered, and core hands them out again
            if !idle {
                break;
            }
            self.build_us += Self::micros(start);
        }
    }

    // tessellates the job into the slot's mesh; false when the game's tessellator is in use
    fn build_replace(&mut self, job: &GhostJob, generation: u64, tessellator: &mut Tessellator, alpha: u8) -> bool {
        let slot = job.slot as usize;
        let mut built = GhostBuilt::default();
        if self.usable {
            built = self.tessellator.build(tessellator, job, &self.blocks, alpha, &mut self.slots[slot]);
        }
        if built.busy {
            return false;
        }
        // the slot's mesh is no longer a group's
        self.group.free(job.slot);
        self.sizes[slot] = job.slot_size;
        self.cells_built += built.cells;
        self.skipped_cells += built.skipped;
        let bytes = built.vertices * Self::GPU_BYTES_PER_VERTEX;
        self.built.push(SlotBuilt { slot: job.slot, serial: job.serial, generation, bytes, ok: built.ok });
        true
    }

    // an Append adds the job's vertices to the group buffer, a Finish uploads the buffer as the slot's mesh; a job
    // applied already is answered as built, any other job core sent out of turn is not answered. False when the
    // game's tessellator is in use
    fn build_group(&mut self, job: &GhostJob, generation: u64, tessellator: &mut Tessellator, alpha: u8) -> bool {
        let slot = job.slot as usize;
        let answer = |bytes: u64, ok: bool| SlotBuilt { slot: job.slot, serial: job.serial, generation, bytes, ok };
        let step = self.group.step(job);
        match step {
            GroupStep::Ignore => return true,
            GroupStep::Repeat => {
                self.built.push(answer(self.group.vertices() * Self::GPU_BYTES_PER_VERTEX, true));
                return true;
            }
            GroupStep::RepeatFinish => {
                self.built.push(answer(self.group.finished_bytes(job.slot), true));
                return true;
            }
            GroupStep::Apply => {}
        }
        if tessellator.is_tessellating() {
            return false;
        }
        if job.group_index == 0 {
            self.group.start(job);
        }
        if job.mode == GhostJobMode::Append {
            let mut built = GhostBuilt::default();
            if self.usable {
                built = self.tessellator.append(tessellator, job, &self.blocks, alpha, &mut self.group);
            }
            if built.busy {
                return false;
            }
            // a failed part fails its group; core starts it again with index 0 or not at all
            if !built.ok {
                self.group.drop_build();
                let reason = if self.usable { "append refused" } else { "renderer not usable" };
                curia::info!("ghost group failed", {
                    slot: job.slot,
                    group_serial: job.group_serial,
                    group_index: job.group_index,
                    reason: reason
                });
            }
            self.cells_built += built.cells;
            self.skipped_cells += built.skipped;
            self.built.push(answer(self.group.vertices() * Self::GPU_BYTES_PER_VERTEX, built.ok));
            return true;
        }
        let start = Instant::now();
        let Some(data) = self.group.data_mut() else { return true };
        let finished = self.tessellator.finish(tessellator, data, &mut self.slots[slot]);
        if finished.busy {
            return false;
        }
        let us = Self::micros(start);
        let bytes = if finished.ok {
            finished.vertices * Self::GPU_BYTES_PER_VERTEX + finished.indices * Self::INDEX_BYTES
        } else {
            0
        };
        if finished.ok {
            self.group.finish(job.slot, job.group_serial, bytes);
            self.sizes[slot] = job.slot_size;
        } else {
            self.group.drop_build();
            curia::info!("ghost group failed", {
                slot: job.slot,
                group_serial: job.group_serial,
                group_index: job.group_index,
                reason: "finish refused"
            });
        }
        self.built.push(answer(bytes, finished.ok));
        curia::info!("ghost group uploaded", {
            slot: job.slot,
            vertices: finished.vertices,
            indices: finished.indices,
            ok: finished.ok,
            us: us
        });
        true
    }

    #[allow(unsafe_code)]
    fn draw(&mut self, take: &GhostTake, screen: &ScreenContext, world: &GhostWorld, camera: [f32; 3]) -> u64 {
        self.culled = 0;
        self.meshes_drawn = 0;
        if world.blend.is_null() || world.atlas.is_null() {
            return 0;
        }
        let start = Instant::now();
        let count = self.cull(take, screen, camera);
        if count > 0 {
            // SAFETY: the level renderer's live atlas texture holds live uses; the copy takes its own for the frame
            let mut texture = unsafe { (*world.atlas).clone_ref() };
            let variant = TextureVariant::new(TexturePtrLayout::VARIANT_INDEX, texture);
            for index in 0..count {
                let draw = take.draws()[self.order[index] as usize];
                let Some(mesh) = self.slots[draw.slot as usize].as_ref() else { continue };
                let offset = std::array::from_fn(|axis| {
                    draw.origin[axis] as f32 - camera[axis] + draw.offset[axis] as f32 * draw.scale
                });
                screen.set_ghost_constants();
                let pushed = screen.push_world_matrix(offset, draw.scale);
                if pushed.is_some() {
                    // SAFETY: the item-in-hand renderer's material and a texture copy that holds its own uses
                    unsafe { mesh.render(screen, world.blend, &variant) };
                }
                drop(pushed);
            }
            // SAFETY: the copy holds the two uses `clone_ref` took; the variant was a byte copy and is not released
            unsafe { texture.release() };
        }
        self.meshes_drawn = count as u32;
        Self::micros(start)
    }

    // fills `order` with the indices of the listed draws that have a mesh and are in view, farthest first; a slot's
    // box is outside when all eight corners lie beyond the same clip plane of projection x view, in camera-relative
    // space. With no camera matrices nothing is culled. Returns how many are in `order` and counts the rest in
    // `culled`
    fn cull(&mut self, take: &GhostTake, screen: &ScreenContext, camera: [f32; 3]) -> usize {
        let clip = screen.clip_matrix();
        let draws = take.draws();
        let mut count = 0;
        for (index, draw) in draws.iter().enumerate() {
            if count >= self.order.len() {
                break;
            }
            let slot = draw.slot as usize;
            if self.slots.get(slot).is_none_or(Option::is_none) {
                continue;
            }
            let size = self.sizes[slot];
            let lo: [f32; 3] = std::array::from_fn(|axis| {
                draw.origin[axis] as f32 - camera[axis] + draw.offset[axis] as f32 * draw.scale
            });
            let hi: [f32; 3] = std::array::from_fn(|axis| lo[axis] + size[axis] as f32 * draw.scale);
            if let Some(clip) = clip.as_ref()
                && Self::outside(clip, lo, hi)
            {
                self.culled += 1;
                continue;
            }
            let centre: [f32; 3] = std::array::from_fn(|axis| (lo[axis] + hi[axis]) * 0.5);
            self.distance[slot] = centre.iter().map(|axis| axis * axis).sum();
            self.order[count] = index as u32;
            count += 1;
        }
        // farthest first; equal distances fall back to the slot, so the order is the same every frame
        let distance = &self.distance;
        self.order[..count].sort_unstable_by(|a, b| {
            let (sa, sb) = (draws[*a as usize].slot, draws[*b as usize].slot);
            let (da, db) = (distance[sa as usize], distance[sb as usize]);
            db.total_cmp(&da).then(sa.cmp(&sb))
        });
        count
    }

    // all eight corners of the box beyond one clip plane
    fn outside(clip: &[[f32; 4]; 4], lo: [f32; 3], hi: [f32; 3]) -> bool {
        let mut outside = [0usize; 6];
        for corner in 0..Self::CORNERS {
            let point = [
                if corner & 1 != 0 { hi[0] } else { lo[0] },
                if corner & 2 != 0 { hi[1] } else { lo[1] },
                if corner & 4 != 0 { hi[2] } else { lo[2] },
                1.0,
            ];
            let p: [f32; 4] = std::array::from_fn(|row| (0..4).map(|column| clip[column][row] * point[column]).sum());
            let checks = [p[0] < -p[3], p[0] > p[3], p[1] < -p[3], p[1] > p[3], p[2] < -p[3], p[2] > p[3]];
            for (plane, beyond) in outside.iter_mut().zip(checks) {
                *plane += usize::from(beyond);
            }
        }
        outside.contains(&Self::CORNERS)
    }

    // the world of this frame, from the client the tick saw last; None out of a world or with no player
    #[allow(unsafe_code)]
    unsafe fn world_of(
        &self,
        entries: &GameEntries,
        client: Option<NonNull<c_void>>,
        renderer: *mut c_void,
    ) -> Option<GhostWorld> {
        // SAFETY: the client the update detour saw, live while the game runs
        let client = unsafe { ClientInstance::from_raw(client?.as_ptr(), entries) }?;
        let region = client.region()?;
        let pose = client.local_player()?.pose()?;
        let [x, y, z] = PlayerSpot::feet_of(pose.pos);
        let atlas = if renderer.is_null() {
            std::ptr::null()
        } else {
            renderer
                .cast::<u8>()
                .wrapping_add(self.calls.atlas_texture as usize)
                .cast::<TexturePtrLayout>()
                .cast_const()
        };
        Some(GhostWorld {
            region: region.as_ptr(),
            feet: amml_game::BlockPos { x, y, z },
            blend: self.blend_material(&client).unwrap_or(std::ptr::null()),
            atlas,
        })
    }

    // the blend-block material of the client's item-in-hand renderer, as a `mce::MaterialPtr const*`
    #[allow(unsafe_code)]
    fn blend_material(&self, client: &ClientInstance) -> Option<*const c_void> {
        // SAFETY: a live ClientInstance; the entry is the slot of `getItemInHandRenderer`
        let function = unsafe { RawObject::slot(client.as_ptr(), self.calls.item_in_hand_renderer_slot) }?;
        // SAFETY: the slot holds `ItemInHandRenderer* ClientInstance::getItemInHandRenderer()`
        let renderer = unsafe { RawObject::call_pointer(function, client.as_ptr()) }?;
        Some(renderer.cast::<u8>().wrapping_add(self.calls.blend_material as usize).cast::<c_void>().cast_const())
    }

    // the camera position of the frame's first view
    #[allow(unsafe_code)]
    unsafe fn camera_of(&self, entries: &GameEntries, frame: *const c_void) -> Option<[f32; 3]> {
        if frame.is_null() {
            return None;
        }
        // SAFETY: the live FrameRenderObject of the call; `mViews` is a std::vector<ViewRenderObject>
        let [first, last] = unsafe { RawObject::read::<[usize; 2]>(frame, self.calls.frame_views) }?;
        if first == 0 || first == last {
            return None;
        }
        // SAFETY: the first ViewRenderObject of the frame
        unsafe { RenderCamera::new(entries).view_position(first as *const c_void) }
    }

    fn micros(start: Instant) -> u64 {
        u64::try_from(start.elapsed().as_micros()).unwrap_or(u64::MAX)
    }
}
