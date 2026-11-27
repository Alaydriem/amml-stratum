use std::sync::Arc;
use std::time::Instant;

use amml_api::amml_abi::{AmmlStatus, BlockEntityPass, WorldRendered};
use amml_api::{
    ApiError, BlockEntities, BlockEntityCamera, BlockEntityGhost, BlockEntityPassExt, BlockGrid, BlockHandle,
    BlockMesher, BlockPos, Blocks, GhostEntityOptions, MeshId, MesherOptions, Transform, Vec3, WorldCanvas,
    WorldRenderedExt,
};
use stratum_core::structs::ghost::{GhostBlockEntity, GhostBlockEntityTable, GhostJob, GhostJobMode, SlotBuilt};

use crate::ghost::{BlockActorCell, BlockActorKey, BlockActorPool, BlockResolver, GroupTracker, SkullPoseMap};
use crate::services::{GhostTake, SharedGhost};
use crate::types::GroupStep;

/// The ghost in the world render: each `WorldRendered` it takes the frame core published, frees and builds the mesh
/// slots it lists with the loader's block mesher, draws the listed slots that are in view see-through over the world,
/// farthest first, and reports what it built and how many it left out of view. What to build and draw is core's; the
/// renderer holds only the slot table of mesh ids and its per-slot scratch, sized once to core's slot capacity, and
/// the palette's block handles.
///
/// Every mesh is built from the job's padded grid, so a mesh point sits one cell above the job's first cell on each
/// axis; a group append is placed the same way, and every draw moves the mesh back by one cell.
///
/// The block-entity cells of each slot's job are kept with the slot and drawn as the loader's block entity ghosts in
/// the game's block-entity pass, for the full-size slots that were in view in the last frame.
///
/// Bound: the slot table and every per-slot vector hold `slot_capacity` entries, which changes only with the config;
/// each slot's block-entity cells are at most the cells of one job; the palette holds one handle per palette id; the
/// ghost pool has its own bound.
pub struct GhostRenderer {
    // the loader epoch of the world the slots and handles belong to; 0 outside a world
    epoch: u64,
    // core's world epoch the slots belong to
    core_epoch: u64,
    usable: bool,
    // a dimension change since the last frame: the next frame joins again
    rejoin: bool,
    // frames since the last try to make a joined world usable
    retry_frames: u32,
    water_tint: bool,
    blocks: BlockResolver,
    slots: Vec<Option<MeshId>>,
    group: GroupTracker,
    generation: u64,
    built: Vec<SlotBuilt>,
    // per slot: the built box in cells, and its squared distance this frame
    sizes: Vec<[u32; 3]>,
    distance: Vec<f32>,
    // this frame's draw indices in view, farthest first
    order: Vec<u32>,
    culled: u32,
    meshes_drawn: u32,
    // set on a join or when the loader dropped the meshes, cleared once a report has carried it
    world_reset: bool,
    palette_resolved: bool,
    build_us: u64,
    cells_built: u32,
    skipped_cells: u32,
    // a build or draw error was logged since the last join, so each kind is logged once per world
    build_error_logged: bool,
    draw_error_logged: bool,
    pool: BlockActorPool<BlockEntityGhost>,
    table: Option<Arc<GhostBlockEntityTable>>,
    entity_cells: Vec<Vec<BlockActorCell>>,
    // the last frame's full-size slots in view that hold block-entity cells, farthest first, each with the world
    // position its job's origin is drawn at
    visible: Vec<(u32, [i32; 3])>,
}

impl GhostRenderer {
    /// What core counts a built mesh's GPU memory at: the measured vertex size on the GPU.
    pub const GPU_BYTES_PER_VERTEX: u64 = 20;
    /// A group mesh's own index buffer holds 32-bit indices.
    pub const INDEX_BYTES: u64 = 4;
    const VERTICES_PER_QUAD: u64 = 4;
    const INDICES_PER_QUAD: u64 = 6;
    // the quads an appended cell is counted at before its group is finished: a full cube
    const QUADS_PER_CELL: u64 = 6;
    const ALPHA_MAX: f32 = 255.0;
    const CORNERS: usize = 8;
    // the frames between two tries to make a joined world usable: its blocks can be found later than the join
    const RETRY_FRAMES: u32 = 60;

    pub fn new(water_tint: bool) -> Self {
        Self {
            epoch: 0,
            core_epoch: 0,
            usable: false,
            rejoin: false,
            retry_frames: 0,
            water_tint,
            blocks: BlockResolver::new(),
            slots: Vec::new(),
            group: GroupTracker::new(),
            generation: 0,
            built: Vec::new(),
            sizes: Vec::new(),
            distance: Vec::new(),
            order: Vec::new(),
            culled: 0,
            meshes_drawn: 0,
            world_reset: false,
            palette_resolved: false,
            build_us: 0,
            cells_built: 0,
            skipped_cells: 0,
            build_error_logged: false,
            draw_error_logged: false,
            pool: BlockActorPool::new(),
            table: None,
            entity_cells: Vec::new(),
            visible: Vec::new(),
        }
    }

    /// One `WorldRendered`: take, free, build, cull and sort, draw, report, in the order the C++ renderer did them.
    /// `epoch` is the loader's world epoch of the event.
    pub fn frame(
        &mut self,
        event: &WorldRendered,
        epoch: u64,
        blocks: &Blocks,
        entities: &BlockEntities,
        ghost: &SharedGhost,
    ) {
        let Some(canvas) = event.canvas() else { return };
        let mesher = event.mesher();
        let Some(take) = ghost.take() else { return };
        // core's epoch moves once per world; the loader's epoch and a dimension change also make the renderer join
        // again, and the join then asks core for a new generation
        let epoch_moved = take.world_epoch() != self.core_epoch;
        self.core_epoch = take.world_epoch();
        let rejoin = std::mem::take(&mut self.rejoin);
        if !take.in_world() {
            if self.joined() {
                self.leave(Some(&canvas), entities);
            }
        } else if epoch_moved || epoch != self.epoch || rejoin {
            if self.joined() {
                self.leave(Some(&canvas), entities);
            }
            self.join(epoch, blocks, !epoch_moved);
        } else if self.joined() && !self.usable {
            self.retry_join(blocks);
        }
        self.apply(&take, &canvas, mesher.as_ref(), blocks, entities);
        let draw_us = self.draw(&take, &canvas);
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

    /// One `BlockEntityPass`: draws the block-entity cells of the last frame's visible full-size slots, for the
    /// player's camera only and only in the world they were built in. Returns how many cells were drawn.
    pub fn block_entity_pass(&mut self, event: &BlockEntityPass, epoch: u64, entities: &BlockEntities) -> u32 {
        // a dimension change is applied by the next `WorldRendered`, which comes after this pass of the same frame
        if event.camera() != BlockEntityCamera::Player
            || self.visible.is_empty()
            || !self.usable
            || self.rejoin
            || epoch != self.epoch
        {
            return 0;
        }
        self.pool.begin_frame();
        let Self { pool, blocks, visible, entity_cells, table, .. } = self;
        let mut drawn = 0;
        let mut stale = false;
        for (slot, base) in visible.iter() {
            let Some(cells) = entity_cells.get(*slot as usize) else { continue };
            for cell in cells {
                let Some(block) = blocks.block(cell.key.palette) else { continue };
                let pos =
                    BlockPos { x: base[0] + cell.local[0], y: base[1] + cell.local[1], z: base[2] + cell.local[2] };
                let key = cell.key;
                let nbt = if key.entry == GhostBlockEntity::NONE {
                    None
                } else {
                    table.as_deref().and_then(|table| table.entry(key.entry))
                };
                let made = pool.get_or_make(key, || Self::create(entities, block, nbt, key), |old| entities.free(old));
                let Some(ghost) = made else { continue };
                match event.draw_block_entity(ghost, pos) {
                    Ok(()) => drawn += 1,
                    Err(ApiError::Stale) => stale = true,
                    Err(_) => {}
                }
            }
        }
        if stale {
            // the loader dropped the ghosts (a device reset or surface loss): they are made again, and core builds
            // the slots again with the meshes
            curia::info!("block entity ghosts stale, made again", {});
            self.pool.clear(|ghost| entities.free(ghost));
            self.world_reset = true;
        }
        drawn
    }

    /// The world was left: the loader has freed every mesh, group and ghost, so the ids are forgotten without a call.
    /// Gives how many meshes the renderer held. Game thread.
    pub fn world_left(&mut self, entities: &BlockEntities) -> usize {
        let held = self.meshes();
        if self.joined() {
            self.leave(None, entities);
        }
        held
    }

    /// The player's dimension changes: the next frame joins again.
    pub fn dimension_changing(&mut self) {
        self.rejoin = true;
    }

    /// The meshes the slot table holds.
    pub fn meshes(&self) -> usize {
        self.slots.iter().filter(|slot| slot.is_some()).count()
    }

    /// The ghosts the pool holds.
    pub fn block_entities(&self) -> usize {
        self.pool.len()
    }

    /// The block of palette id `id` in the joined world, for easy place's creative pick; None outside a world and for
    /// an id the palette does not resolve.
    pub fn palette_block(&self, id: u16) -> Option<BlockHandle> {
        if !self.usable {
            return None;
        }
        self.blocks.block(id)
    }

    /// Drops every mesh and handle, as a world exit does; the next frame in a world joins again. The loader frees the
    /// meshes and ghosts of a disabled mod itself.
    pub fn reset(&mut self, entities: &BlockEntities) {
        self.leave(None, entities);
        self.core_epoch = 0;
    }

    fn joined(&self) -> bool {
        self.epoch != 0
    }

    fn create(
        entities: &BlockEntities,
        block: BlockHandle,
        nbt: Option<&[u8]>,
        key: BlockActorKey,
    ) -> Result<BlockEntityGhost, bool> {
        let chest_pair = key.chest.map(|pair| {
            let [x, y, z] = pair.offset();
            BlockPos { x, y, z }
        });
        entities.create(block, nbt, GhostEntityOptions { chest_pair, skull: key.skull }).map_err(|error| {
            // the game made no actor, or the block cannot hold the options: the same key fails again; any other
            // refusal (the live cap before the frees are released, no world yet) is tried again later
            let for_good = matches!(
                error,
                ApiError::Loader(status) if status == AmmlStatus::FAILED || status == AmmlStatus::INVALID_ARGUMENT
            );
            curia::info!("block entity ghost not made", { error: format!("{error:?}"), for_good: for_good });
            for_good
        })
    }

    fn join(&mut self, epoch: u64, blocks: &Blocks, ask_reset: bool) {
        self.epoch = epoch;
        // the meshes and handles are gone; unless core started a new generation for this join already, it starts one
        // with the palette and a clear
        self.world_reset = self.world_reset || ask_reset;
        self.usable = self.blocks.join(blocks);
        self.retry_frames = 0;
        self.build_error_logged = false;
        self.draw_error_logged = false;
        curia::info!("ghost renderer joined the world", { usable: self.usable, epoch: epoch });
    }

    // a joined world whose air was not found is tried again every `RETRY_FRAMES` frames; once it is usable core starts
    // a new generation, so the palette comes again and is resolved
    fn retry_join(&mut self, blocks: &Blocks) {
        self.retry_frames += 1;
        if self.retry_frames < Self::RETRY_FRAMES {
            return;
        }
        self.retry_frames = 0;
        self.usable = self.blocks.join(blocks);
        if self.usable {
            self.world_reset = true;
            curia::info!("ghost renderer usable after a retry", { epoch: self.epoch });
        }
    }

    // a mesh id and a block handle belong to the world that was left; with a canvas the meshes are freed, without one
    // the loader has freed them already
    fn leave(&mut self, canvas: Option<&WorldCanvas<'_>>, entities: &BlockEntities) {
        for slot in self.slots.iter_mut() {
            if let (Some(mesh), Some(canvas)) = (slot.take(), canvas) {
                let _ = canvas.free(mesh);
            }
        }
        self.entity_cells.iter_mut().for_each(Vec::clear);
        self.visible.clear();
        self.pool.clear(|ghost| entities.free(ghost));
        self.table = None;
        self.group.clear();
        self.blocks.leave();
        self.epoch = 0;
        self.usable = false;
    }

    fn free_slot(&mut self, slot: usize, canvas: &WorldCanvas<'_>) {
        if let Some(mesh) = self.slots.get_mut(slot).and_then(Option::take) {
            let _ = canvas.free(mesh);
        }
    }

    // in core's order: free every slot, then the frees, then the palette, then the jobs
    fn apply(
        &mut self,
        take: &GhostTake,
        canvas: &WorldCanvas<'_>,
        mesher: Option<&BlockMesher<'_>>,
        blocks: &Blocks,
        entities: &BlockEntities,
    ) {
        let capacity = take.slot_capacity() as usize;
        if capacity != self.slots.len() {
            // sized once to core's slot capacity, which changes only with the config
            for slot in 0..self.slots.len() {
                self.free_slot(slot, canvas);
            }
            self.slots.clear();
            self.slots.resize_with(capacity, || None);
            self.built.clear();
            self.built.reserve(capacity);
            self.sizes = vec![[0; 3]; capacity];
            self.distance = vec![0.0; capacity];
            self.order = vec![0; capacity];
            self.entity_cells.clear();
            self.entity_cells.resize_with(capacity, Vec::new);
            self.visible = Vec::with_capacity(capacity);
            self.group.resize(capacity);
        }
        // a group build of an earlier generation is never finished
        if take.clear_all() || take.generation() != self.generation {
            self.group.clear();
        }
        self.generation = take.generation();
        if take.clear_all() {
            for slot in 0..self.slots.len() {
                self.free_slot(slot, canvas);
            }
            self.entity_cells.iter_mut().for_each(Vec::clear);
        }
        for free in take.frees() {
            let slot = *free as usize;
            if slot < self.slots.len() {
                self.free_slot(slot, canvas);
                self.group.free(*free);
            }
            if let Some(cells) = self.entity_cells.get_mut(slot) {
                cells.clear();
            }
        }
        self.palette_resolved = false;
        if let Some(palette) = take.palette() {
            self.palette_resolved = true;
            // the ghosts' keys and every slot's block-entity cells name palette ids and NBT entries of the table that
            // is gone; the jobs of this take fill the cells again
            self.table = take.shared_block_entities();
            self.pool.clear(|ghost| entities.free(ghost));
            self.entity_cells.iter_mut().for_each(Vec::clear);
            self.visible.clear();
            let start = Instant::now();
            self.blocks.resolve(blocks, palette, take.rotation());
            curia::info!("ghost palette resolved", { us: Self::micros(start) });
        }
        self.build(take, canvas, mesher);
    }

    fn build(&mut self, take: &GhostTake, canvas: &WorldCanvas<'_>, mesher: Option<&BlockMesher<'_>>) {
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
            if job.mode == GhostJobMode::Replace {
                self.build_replace(job, *generation, canvas, mesher, alpha);
            } else {
                self.build_group(job, *generation, canvas, mesher, alpha);
            }
            self.build_us += Self::micros(start);
        }
    }

    // the mesher grid of a job: its padded cells, the world position of the padded corner and the palette handles
    fn grid<'a>(job: &'a GhostJob, palette: &'a [BlockHandle]) -> BlockGrid<'a> {
        let [x, y, z] = job.origin;
        BlockGrid {
            dims: job.dims.map(|side| side + 2),
            origin: BlockPos { x: x - 1, y: y - 1, z: z - 1 },
            cells: &job.cells,
            palette,
        }
    }

    // the cells of the job's part that hold a block, padding excluded
    fn filled_cells(job: &GhostJob) -> u32 {
        let [sx, sy, sz] = job.dims.map(|side| side as usize);
        let (px, pz) = (sx + 2, sz + 2);
        let mut count = 0;
        for (index, cell) in job.cells.iter().enumerate() {
            if *cell == BlockResolver::EMPTY {
                continue;
            }
            let (x, rest) = (index % px, index / px);
            let (z, y) = (rest % pz, rest / pz);
            if x != 0 && y != 0 && z != 0 && x <= sx && y <= sy && z <= sz {
                count += 1;
            }
        }
        count
    }

    fn options(&self, job: &GhostJob, alpha: u8) -> MesherOptions {
        MesherOptions { alpha, skip_matching_world: job.skip_matching, water_tint: self.water_tint }
    }

    fn log_build_error(&mut self, slot: u32, error: ApiError) {
        if !self.build_error_logged {
            self.build_error_logged = true;
            curia::warn!("ghost build refused", { slot: slot, error: format!("{error:?}") });
        }
    }

    // builds the job into the slot's mesh, in place of the slot's last mesh
    fn build_replace(
        &mut self,
        job: &GhostJob,
        generation: u64,
        canvas: &WorldCanvas<'_>,
        mesher: Option<&BlockMesher<'_>>,
        alpha: u8,
    ) {
        let slot = job.slot as usize;
        let built = match mesher.filter(|_| self.usable) {
            Some(mesher) => mesher.build(&Self::grid(job, self.blocks.handles()), &self.options(job, alpha)),
            None => Err(ApiError::NotReady),
        };
        // a build the world could not make yet keeps the slot's last mesh; core builds it again
        if built != Err(ApiError::NotReady) {
            self.free_slot(slot, canvas);
            // the slot's mesh is no longer a group's
            self.group.free(job.slot);
        }
        let (ok, bytes, skipped) = match built {
            Ok(built) => {
                if built.quads > 0 {
                    self.slots[slot] = Some(built.mesh);
                }
                let bytes = u64::from(built.quads) * Self::VERTICES_PER_QUAD * Self::GPU_BYTES_PER_VERTEX;
                (true, bytes, built.skipped)
            }
            Err(error) => {
                self.log_build_error(job.slot, error);
                (false, 0, Vec::new())
            }
        };
        self.keep_entity_cells(job, ok, &skipped);
        self.sizes[slot] = job.slot_size;
        if ok {
            let skipped_count = u32::try_from(skipped.len()).unwrap_or(u32::MAX);
            self.cells_built += Self::filled_cells(job).saturating_sub(skipped_count);
            self.skipped_cells += skipped_count;
        }
        self.built.push(SlotBuilt { slot: job.slot, serial: job.serial, generation, bytes, ok });
    }

    // an Append adds the job's cells to the open group, a Finish makes the group the slot's mesh; a job applied
    // already is answered as built, any other job core sent out of turn is not answered
    fn build_group(
        &mut self,
        job: &GhostJob,
        generation: u64,
        canvas: &WorldCanvas<'_>,
        mesher: Option<&BlockMesher<'_>>,
        alpha: u8,
    ) {
        let slot = job.slot as usize;
        let answer = |bytes: u64, ok: bool| SlotBuilt { slot: job.slot, serial: job.serial, generation, bytes, ok };
        match self.group.step(job) {
            GroupStep::Ignore => return,
            GroupStep::Repeat => {
                self.built.push(answer(self.group.bytes(), true));
                return;
            }
            GroupStep::RepeatFinish => {
                self.built.push(answer(self.group.finished_bytes(job.slot), true));
                return;
            }
            GroupStep::Apply => {}
        }
        let Some(mesher) = mesher.filter(|_| self.usable) else {
            self.group.drop_build();
            self.built.push(answer(0, false));
            return;
        };
        if job.group_index == 0 {
            match mesher.begin_group(job.slot_size) {
                Ok(group) => self.group.start(job, group),
                Err(error) => {
                    self.log_build_error(job.slot, error);
                    self.built.push(answer(0, false));
                    return;
                }
            }
        }
        if job.mode == GhostJobMode::Append {
            self.append(job, mesher, alpha, answer);
            return;
        }
        let start = Instant::now();
        let finished = match self.group.take() {
            Some(group) => mesher.finish(group),
            None => Err(ApiError::Stale),
        };
        let us = Self::micros(start);
        match finished {
            Ok(built) => {
                self.free_slot(slot, canvas);
                if built.quads > 0 {
                    self.slots[slot] = Some(built.mesh);
                }
                let quads = u64::from(built.quads);
                let bytes = quads * Self::VERTICES_PER_QUAD * Self::GPU_BYTES_PER_VERTEX
                    + quads * Self::INDICES_PER_QUAD * Self::INDEX_BYTES;
                self.group.finish(job.slot, job.group_serial, bytes);
                self.sizes[slot] = job.slot_size;
                // a group mesh draws no block-entity models
                if let Some(cells) = self.entity_cells.get_mut(slot) {
                    cells.clear();
                }
                self.built.push(answer(bytes, true));
                curia::info!("ghost group uploaded", { slot: job.slot, quads: built.quads, us: us });
            }
            Err(error) => {
                self.group.drop_build();
                curia::info!("ghost group failed", {
                    slot: job.slot,
                    group_serial: job.group_serial,
                    group_index: job.group_index,
                    reason: format!("finish {error:?}")
                });
                self.built.push(answer(0, false));
            }
        }
    }

    // adds the job's cells to the open group, each cell one cell above its place in the slot box; a failed part fails
    // its group, and core starts it again with index 0 or not at all
    fn append(&mut self, job: &GhostJob, mesher: &BlockMesher<'_>, alpha: u8, answer: impl Fn(u64, bool) -> SlotBuilt) {
        let [x, y, z] = job.group_offset;
        let offset = BlockPos { x, y, z };
        let options = self.options(job, alpha);
        let grid = Self::grid(job, self.blocks.handles());
        let appended = match self.group.group_mut() {
            Some(group) => mesher.append(group, &grid, offset, &options),
            None => Err(ApiError::Stale),
        };
        match appended {
            Ok(()) => {
                let cells = Self::filled_cells(job);
                self.cells_built += cells;
                let bytes =
                    u64::from(cells) * Self::QUADS_PER_CELL * Self::VERTICES_PER_QUAD * Self::GPU_BYTES_PER_VERTEX;
                self.group.appended(bytes);
                self.built.push(answer(self.group.bytes(), true));
            }
            Err(error) => {
                self.group.drop_build();
                curia::info!("ghost group failed", {
                    slot: job.slot,
                    group_serial: job.group_serial,
                    group_index: job.group_index,
                    reason: format!("append {error:?}")
                });
                self.built.push(answer(self.group.bytes(), false));
            }
        }
    }

    // the job's block-entity cells in the slot's list, without the cells the mesher left out as matching the world
    // and the cells outside the job's part; the list is emptied when `keep` is false
    fn keep_entity_cells(&mut self, job: &GhostJob, keep: bool, skipped: &[u32]) {
        let Some(cells) = self.entity_cells.get_mut(job.slot as usize) else { return };
        cells.clear();
        if !keep {
            return;
        }
        let [sx, sy, sz] = job.dims.map(|side| side as usize);
        let (px, pz) = (sx + 2, sz + 2);
        for entity in &job.block_entities {
            if skipped.binary_search(&entity.cell).is_ok() {
                continue;
            }
            let index = entity.cell as usize;
            let Some(palette) = job.cells.get(index).copied() else { continue };
            if palette == BlockResolver::EMPTY {
                continue;
            }
            // the padded grid runs X fastest, then Z, then Y, one cell of border on each side
            let (x, rest) = (index % px, index / px);
            let (z, y) = (rest % pz, rest / pz);
            if x == 0 || y == 0 || z == 0 || x > sx || y > sy || z > sz {
                continue;
            }
            let skull = entity.skull.and_then(SkullPoseMap::pose);
            let key = BlockActorKey::new(palette, entity.entry, entity.chest, skull);
            cells.push(BlockActorCell::new(key, [x as i32 - 1, y as i32 - 1, z as i32 - 1]));
        }
    }

    fn draw(&mut self, take: &GhostTake, canvas: &WorldCanvas<'_>) -> u64 {
        self.culled = 0;
        self.meshes_drawn = 0;
        self.visible.clear();
        if !self.usable {
            return 0;
        }
        let start = Instant::now();
        let camera = canvas.camera_position().map_or([0.0; 3], |camera| [camera.x, camera.y, camera.z]);
        let count = self.cull(take, canvas.clip_matrix().as_ref(), camera);
        self.keep_visible(take, count);
        let mut drawn = 0;
        let mut stale = false;
        for index in 0..count {
            let draw = take.draws()[self.order[index] as usize];
            let Some(mesh) = self.slots.get(draw.slot as usize).copied().flatten() else { continue };
            let back = |axis: usize| (draw.offset[axis] - 1) as f32 * draw.scale;
            let transform = Transform {
                origin: BlockPos { x: draw.origin[0], y: draw.origin[1], z: draw.origin[2] },
                offset: Vec3 { x: back(0), y: back(1), z: back(2) },
                scale: draw.scale,
                yaw_degrees: 0.0,
            };
            match canvas.draw(mesh, transform) {
                Ok(()) => drawn += 1,
                Err(ApiError::Stale) => stale = true,
                Err(ApiError::NotReady) => {}
                Err(error) => {
                    if !self.draw_error_logged {
                        self.draw_error_logged = true;
                        curia::warn!("ghost draw refused", { slot: draw.slot, error: format!("{error:?}") });
                    }
                }
            }
        }
        if stale {
            // the loader dropped the meshes (a device reset or a graphics mode switch): core builds them again
            curia::info!("ghost meshes stale, built again", {});
            self.slots.iter_mut().for_each(|slot| *slot = None);
            self.group.clear();
            self.world_reset = true;
        }
        self.meshes_drawn = drawn;
        Self::micros(start)
    }

    // the full-size slots of the first `count` entries of `order` that hold block-entity cells, in that order; a
    // miniature draws no block-entity models
    fn keep_visible(&mut self, take: &GhostTake, count: usize) {
        let draws = take.draws();
        for index in &self.order[..count] {
            let Some(draw) = draws.get(*index as usize) else { continue };
            if (draw.scale - 1.0).abs() > f32::EPSILON
                || self.entity_cells.get(draw.slot as usize).is_none_or(Vec::is_empty)
            {
                continue;
            }
            let base = std::array::from_fn(|axis| draw.origin[axis] + draw.offset[axis]);
            self.visible.push((draw.slot, base));
        }
    }

    // fills `order` with the indices of the listed draws that have a mesh or block-entity cells and are in view,
    // farthest first; a slot's box is outside when all eight corners lie beyond the same clip plane of projection x
    // view, in camera-relative space. With no clip matrix nothing is culled. Returns how many are in `order` and counts
    // the rest in `culled`
    fn cull(&mut self, take: &GhostTake, clip: Option<&[[f32; 4]; 4]>, camera: [f32; 3]) -> usize {
        let draws = take.draws();
        let mut count = 0;
        for (index, draw) in draws.iter().enumerate() {
            if count >= self.order.len() {
                break;
            }
            let slot = draw.slot as usize;
            let has_mesh = self.slots.get(slot).is_some_and(Option::is_some);
            let has_entities = self.entity_cells.get(slot).is_some_and(|cells| !cells.is_empty());
            if !has_mesh && !has_entities {
                continue;
            }
            let size = self.sizes[slot];
            let lo: [f32; 3] = std::array::from_fn(|axis| {
                draw.origin[axis] as f32 - camera[axis] + draw.offset[axis] as f32 * draw.scale
            });
            let hi: [f32; 3] = std::array::from_fn(|axis| lo[axis] + size[axis] as f32 * draw.scale);
            if let Some(clip) = clip
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

    fn micros(start: Instant) -> u64 {
        u64::try_from(start.elapsed().as_micros()).unwrap_or(u64::MAX)
    }
}
