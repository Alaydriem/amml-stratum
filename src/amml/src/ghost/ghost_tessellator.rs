use std::ffi::c_void;

use amml_game::BlockPos;
use amml_msvc::MsvcHeap;
use stratum_core::structs::ghost::GhostJob;

use crate::game::{
    BlockCalls, GameBlockTessellator, GameVec, MeshDataLayout, OpenBuild, OwnedMesh, OwnedMeshData, Tessellator,
};
use crate::ghost::{BlockResolver, GhostGroupBuffer};
use crate::types::{GhostBuilt, GhostCalls, GroupFinished, PrimitiveMode};

/// Builds one ghost job into an `mce::Mesh`, or into the mini ghost group buffer, with the game's `BlockTessellator`.
/// Neighbours come from the job's padded grid through the tessellator's 20 x 20 x 20 block cache window, so ghost
/// blocks connect to each other and not to the world; a tinted block the tessellator leaves grey takes the game's
/// biome colour at its world position. Owns one `BlockTessellator` for each world, made again on every join; the
/// last one is kept until the next join, like the renderer that holds it. Every begin it opens on the game's
/// tessellator is ended before it returns, also when a build fails part way. Render thread only.
pub struct GhostTessellator {
    calls: GhostCalls,
    game: BlockCalls,
    heap: MsvcHeap,
    blocks: Option<GameBlockTessellator>,
    region: *mut c_void,
    // the axis order of the window index, an index into PERMS; None when the probe found none
    perm: Option<usize>,
    // how far mPos sits inside the window's minimum corner
    offset: i32,
    // +1 when the cross product of a quad's first two edges points out of the block, -1 when it points in
    winding: f32,
    // the block type of this world's full grass block, which takes the biome tint on its top face only
    grass_type: *const c_void,
}

impl GhostTessellator {
    const WINDOW: i32 = 20;
    const BORDER: i32 = 2;
    const VERTS_PER_CELL: usize = 24;
    const QUAD: usize = 4;
    const INDICES_PER_QUAD: usize = 6;
    const MIN_NORMAL: f32 = 1e-8;
    /// How far each quad moves into its own block, so a real face over a ghost face always wins.
    const PUSH: f32 = 0.01;
    /// Any light value the cache holds is taken as computed, so the ghost is lit at full sky and block light.
    const FULL_LIGHT: u16 = 0x0F0F;
    /// The axis orders tried for the window index; the first axis is the slowest.
    const PERMS: [[usize; 3]; 6] = [[0, 1, 2], [0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0]];
    const PERM_NAMES: [&'static str; 6] = ["xyz", "xzy", "yxz", "yzx", "zxy", "zyx"];
    /// The probe cell inside the window; distinct values, so no wrong axis order lands on the same index.
    const PROBE: [i32; 3] = [3, 6, 9];
    /// A quad faces up when its outward normal is within 60 degrees of +y.
    const UP_COS: f32 = 0.5;

    pub fn new(calls: &GhostCalls) -> Self {
        Self {
            calls: *calls,
            game: BlockCalls::new(calls),
            heap: MsvcHeap::new(calls.heap_new, calls.heap_delete),
            blocks: None,
            region: std::ptr::null_mut(),
            perm: None,
            offset: 0,
            winding: 1.0,
            grass_type: std::ptr::null(),
        }
    }

    /// Makes a new block tessellator for `region` in place of the last world's one, finds the cache window's layout
    /// and the quad winding; false when the window layout is not the one the game was measured with, so nothing can
    /// be built in this world. The old block tessellator is pointed at the live region first, so its destructor reads
    /// no freed `BlockSource`.
    ///
    /// # Safety
    /// `region` must be the live client region, and `tessellator` the frame's game tessellator.
    #[allow(unsafe_code)]
    pub unsafe fn join(
        &mut self,
        region: *mut c_void,
        blocks: &BlockResolver,
        tessellator: &mut Tessellator,
        around: BlockPos,
    ) -> bool {
        self.leave();
        if let Some(mut old) = self.blocks.take() {
            // SAFETY: the caller's live region
            unsafe { old.set_region(region) };
            drop(old);
        }
        // SAFETY: the caller's live region; the game heap through the calls
        let Some(mut made) = (unsafe { GameBlockTessellator::new(region, &self.calls) }) else {
            curia::error!("ghost block tessellator not allocated, the ghost is not built in this world", {});
            return false;
        };
        made.set_cached_calculation(true);
        self.blocks = Some(made);
        self.region = region;
        // SAFETY: a block the registry gave for this world, or null
        self.grass_type = unsafe { self.game.block_type(blocks.registry().by_name("minecraft:grass_block")) };
        self.probe_cache(blocks, around);
        if self.perm.is_some() {
            self.calibrate_winding(tessellator, blocks, BlockPos { x: around.x, y: around.y + 2, z: around.z });
        }
        self.ready()
    }

    /// Forgets the world that was left. Touches only this object's own fields: the block tessellator still points at
    /// the old region, which may be freed by now, and is pointed at the next one by the next join.
    pub fn leave(&mut self) {
        self.grass_type = std::ptr::null();
        self.region = std::ptr::null_mut();
        self.perm = None;
        self.offset = 0;
        self.winding = 1.0;
    }

    pub fn ready(&self) -> bool {
        self.blocks.is_some() && self.perm.is_some()
    }

    /// Tessellates every drawn cell of `job` into `out`, positions relative to the job's origin, each quad pushed
    /// inward along its normal, with `alpha` in every vertex colour's alpha byte. `out` is empty when no cell was
    /// drawn. A job larger than one window builds nothing and is not ok; a tessellator in use builds nothing, is
    /// busy and leaves `out` as it was.
    pub fn build(
        &mut self,
        tessellator: &mut Tessellator,
        job: &GhostJob,
        blocks: &BlockResolver,
        alpha: u8,
        out: &mut Option<OwnedMesh>,
    ) -> GhostBuilt {
        let mut built = GhostBuilt::default();
        if tessellator.is_tessellating() {
            built.busy = true;
            return built;
        }
        *out = None;
        if !self.ready() || !Self::fits(job) {
            return built;
        }
        let reserve = self.prepare(job, blocks);
        tessellator.begin_quads(reserve);
        let mut open = OpenBuild::new(tessellator, "stratum_ghost");
        self.tessellate(open.tessellator(), job, blocks, alpha, &mut built);
        built.vertices = open.tessellator().vertex_count() as u64;
        open.tessellator().shift_positions([job.origin[0] as f32, job.origin[1] as f32, job.origin[2] as f32]);
        let mesh = open.finish();
        built.ok = built.vertices == 0 || mesh.as_ref().is_some_and(OwnedMesh::is_valid);
        // a job whose every cell was empty or skipped draws nothing
        if built.vertices > 0 && built.ok {
            *out = mesh;
        }
        built
    }

    /// Tessellates `job` as `build` does, positions relative to the job's origin and then moved by its group offset,
    /// and adds the vertices to `group` in place of making a mesh; the game's tessellator is cleared after the copy,
    /// and ended when the clear left it open. Not ok when the job does not fit or the group buffer refuses the
    /// vertices; busy as `build`.
    pub fn append(
        &mut self,
        tessellator: &mut Tessellator,
        job: &GhostJob,
        blocks: &BlockResolver,
        alpha: u8,
        group: &mut GhostGroupBuffer,
    ) -> GhostBuilt {
        let mut built = GhostBuilt::default();
        if tessellator.is_tessellating() {
            built.busy = true;
            return built;
        }
        if !self.ready() || !Self::fits(job) {
            return built;
        }
        let reserve = self.prepare(job, blocks);
        tessellator.begin_quads(reserve);
        let mut open = OpenBuild::new(tessellator, "stratum_ghost_group_part");
        self.tessellate(open.tessellator(), job, blocks, alpha, &mut built);
        built.vertices = open.tessellator().vertex_count() as u64;
        let shift = std::array::from_fn(|axis| (job.origin[axis] - job.group_offset[axis]) as f32);
        open.tessellator().shift_positions(shift);
        Self::fit_aabb(open.tessellator().mesh_data());
        built.ok = group.add(open.tessellator().mesh_data());
        // the copy is made, so the game's tessellator is free again in this frame
        open.tessellator().clear();
        // the end leaves the tessellator free whatever the clear left; its empty mesh is dropped at once
        open.discard();
        built
    }

    /// Uploads the merged vertices in `data` into `out` as one triangle list with its own 32-bit index buffer, six
    /// indices per quad. `data` is moved into the mesh; `out` is empty when `data` holds no whole quad. A tessellator
    /// in use uploads nothing, is busy and leaves `data` and `out` as they were.
    #[allow(unsafe_code)]
    pub fn finish(
        &mut self,
        tessellator: &mut Tessellator,
        data: &mut OwnedMeshData,
        out: &mut Option<OwnedMesh>,
    ) -> GroupFinished {
        let mut finished = GroupFinished::default();
        if tessellator.is_tessellating() {
            finished.busy = true;
            return finished;
        }
        *out = None;
        let layout = data.layout_mut();
        let vertices = layout.positions.len();
        let quads = vertices / Self::QUAD;
        finished.vertices = vertices as u64;
        let heap = self.heap;
        // SAFETY: the group's vectors grow on the game heap only, which the game's destructor frees them to
        if unsafe { GameVec::resize(&heap, &mut layout.indices, quads * Self::INDICES_PER_QUAD, 0u32) }.is_err() {
            return finished;
        }
        // each quad as two triangles that keep its vertex order, so they face the way the quad-list meshes face
        let (quads_of_indices, _) =
            GameVec::as_mut_slice(&mut layout.indices).as_chunks_mut::<{ Self::INDICES_PER_QUAD }>();
        for (quad, indices) in quads_of_indices.iter_mut().enumerate() {
            let first = (quad * Self::QUAD) as u32;
            indices.copy_from_slice(&[first, first + 1, first + 2, first, first + 2, first + 3]);
        }
        finished.indices = layout.indices.len() as u64;
        layout.mode = PrimitiveMode::TriangleList as u8;
        // a group whose every cell was empty or skipped draws nothing
        if finished.indices == 0 {
            finished.ok = true;
            return finished;
        }
        // the game's own end path, as for every other ghost mesh: the merged vertices take the place of what a
        // tessellation would have left, with the vertex count it would have counted
        tessellator.begin_triangles(0);
        let mut open = OpenBuild::new(tessellator, "stratum_ghost_group");
        let target: *mut MeshDataLayout = open.tessellator().mesh_data();
        // SAFETY: the game tessellator's own live mesh data
        unsafe { data.move_into(target) };
        open.tessellator().set_count(u32::try_from(vertices).unwrap_or(u32::MAX));
        let mesh = open.finish();
        finished.ok = mesh.as_ref().is_some_and(OwnedMesh::is_valid);
        if finished.ok {
            *out = mesh;
        }
        finished
    }

    // a job fits one window when each side plus its 2-cell borders does, and its grid is dims + 2 on each axis
    fn fits(job: &GhostJob) -> bool {
        let [sx, sy, sz] = job.dims.map(|side| side as i32);
        let sides_fit = [sx, sy, sz].iter().all(|side| *side > 0 && side + 2 * Self::BORDER <= Self::WINDOW);
        sides_fit && job.cells.len() == ((sx + 2) * (sy + 2) * (sz + 2)) as usize
    }

    // fills the window for the job and gives the vertex count to reserve: 24 per cell with a block
    fn prepare(&mut self, job: &GhostJob, blocks: &BlockResolver) -> i32 {
        let origin = BlockPos { x: job.origin[0], y: job.origin[1], z: job.origin[2] };
        let [sx, sy, sz] = job.dims.map(|side| side as i32);
        self.fill_window(Some(job), blocks, origin, [sx, sy, sz]);
        let cells = job.cells.iter().filter(|id| !blocks.block(**id).is_null()).count();
        i32::try_from(cells * Self::VERTS_PER_CELL).unwrap_or(i32::MAX)
    }

    // tessellates every drawn cell of `job` into the game's tessellator inside the open begin, in world positions
    #[allow(unsafe_code)]
    fn tessellate(
        &mut self,
        tessellator: &mut Tessellator,
        job: &GhostJob,
        blocks: &BlockResolver,
        alpha: u8,
        built: &mut GhostBuilt,
    ) {
        let region = self.region;
        let winding = self.winding;
        let grass_type = self.grass_type;
        let game = self.game;
        let Some(block_tessellator) = self.blocks.as_mut() else { return };
        let [sx, sy, sz] = job.dims.map(|side| side as usize);
        let (px, pz) = (sx + 2, sz + 2);
        for y in 0..sy {
            for z in 0..sz {
                for x in 0..sx {
                    let id = job.cells[((y + 1) * pz + (z + 1)) * px + (x + 1)];
                    let block = blocks.block(id);
                    if block.is_null() || block == blocks.air() {
                        continue;
                    }
                    let pos = BlockPos {
                        x: job.origin[0] + x as i32,
                        y: job.origin[1] + y as i32,
                        z: job.origin[2] + z as i32,
                    };
                    // the world already holds a block of this name here. The cell stays in the cache window, so its
                    // neighbours still connect to it
                    if job.skip_matching {
                        // SAFETY: the live client region the tessellator was joined with
                        let world = unsafe { game.world_block(region, pos) };
                        // SAFETY: the block the region holds and a block of the same world
                        if unsafe { game.block_type(world) == game.block_type(block) } {
                            built.skipped += 1;
                            continue;
                        }
                    }
                    built.cells += 1;
                    block_tessellator.set_rendering_layer(blocks.layer(id));
                    let first = tessellator.vertex_count();
                    // SAFETY: a block of the joined world and the live region it belongs to
                    unsafe { block_tessellator.tessellate(tessellator, block, pos) };
                    let data = tessellator.mesh_data();
                    // SAFETY: as above
                    unsafe { Self::tint(&game, data, first, block, region, pos, grass_type, winding) };
                    Self::post(data, first, alpha, winding);
                }
            }
        }
    }

    // sets one window entry to stone, asks BlockTessellatorCache::getBlock for the position each layout maps it to,
    // and keeps the layout that answers stone
    fn probe_cache(&mut self, blocks: &BlockResolver, around: BlockPos) {
        self.perm = None;
        self.offset = 0;
        let inner = Self::WINDOW - 2 * Self::BORDER;
        self.fill_window(None, blocks, around, [inner, inner, inner]);
        let window = BlockPos { x: around.x - Self::BORDER, y: around.y - Self::BORDER, z: around.z - Self::BORDER };
        let query =
            BlockPos { x: window.x + Self::PROBE[0], y: window.y + Self::PROBE[1], z: window.z + Self::PROBE[2] };
        let Some(block_tessellator) = self.blocks.as_mut() else { return };
        'probe: for offset in [0, Self::BORDER] {
            for perm in 0..Self::PERMS.len() {
                let index = Self::index_for(perm, Self::PROBE);
                let cache = block_tessellator.cache();
                cache.pos = [window.x + offset, window.y + offset, window.z + offset];
                cache.blocks[index] = blocks.stone();
                let got = block_tessellator.cache_block(query);
                block_tessellator.cache().blocks[index] = blocks.air();
                if got == blocks.stone() {
                    self.perm = Some(perm);
                    self.offset = offset;
                    break 'probe;
                }
            }
        }
        match self.perm {
            Some(perm) => {
                curia::info!("ghost cache window layout", { order: Self::PERM_NAMES[perm], offset: self.offset })
            }
            None => curia::error!("ghost cache window layout not found, the ghost is not built in this world", {}),
        }
    }

    // fills the part of the window that a box of `size` cells at `min` and its 2-cell border cover: from the job's
    // padded grid where it reaches, air elsewhere, or air everywhere when `job` is None. The rest of the window is
    // not read
    fn fill_window(&mut self, job: Option<&GhostJob>, blocks: &BlockResolver, min: BlockPos, size: [i32; 3]) {
        let perm = self.perm.unwrap_or(0);
        let (offset, region) = (self.offset, self.region);
        let Some(block_tessellator) = self.blocks.as_mut() else { return };
        let cache = block_tessellator.cache();
        let window = [min.x - Self::BORDER, min.y - Self::BORDER, min.z - Self::BORDER];
        cache.pos = window.map(|corner| corner + offset);
        cache.region = region;
        cache.air_block = blocks.air();
        cache.light_colors.fill(Self::FULL_LIGHT);
        let reach = size.map(|side| (side + 2 * Self::BORDER).min(Self::WINDOW));
        // the padded grid starts one cell inside the window's corner and is dims + 2 cells on each axis
        let padded = job.map_or([0; 3], |job| job.dims.map(|side| side as i32 + 2));
        for dx in 0..reach[0] {
            for dy in 0..reach[1] {
                for dz in 0..reach[2] {
                    let [gx, gy, gz] = [dx - 1, dy - 1, dz - 1];
                    let mut block = blocks.air();
                    let mut graphics = blocks.air_graphics();
                    if let Some(job) = job
                        && gx >= 0
                        && gy >= 0
                        && gz >= 0
                        && gx < padded[0]
                        && gy < padded[1]
                        && gz < padded[2]
                    {
                        let id = job.cells[((gy * padded[2] + gz) * padded[0] + gx) as usize];
                        let ghost = blocks.block(id);
                        if !ghost.is_null() {
                            block = ghost;
                            graphics = blocks.graphics(id);
                        }
                    }
                    let index = Self::index_for(perm, [dx, dy, dz]);
                    cache.blocks[index] = block;
                    cache.extra_blocks[index] = blocks.air();
                    cache.block_graphics[0][index] = graphics;
                    cache.block_graphics[1][index] = blocks.air_graphics();
                }
            }
        }
    }

    // the window index of a cell for an axis order
    fn index_for(perm: usize, d: [i32; 3]) -> usize {
        let [a, b, c] = Self::PERMS[perm];
        ((d[a] * Self::WINDOW + d[b]) * Self::WINDOW + d[c]) as usize
    }

    // tessellates one stone block and counts whether a quad's cross product points out of it or into it; the probe
    // mesh is dropped at once
    #[allow(unsafe_code)]
    fn calibrate_winding(&mut self, tessellator: &mut Tessellator, blocks: &BlockResolver, pos: BlockPos) {
        if tessellator.is_tessellating() || self.blocks.is_none() {
            return;
        }
        self.fill_window(None, blocks, pos, [1, 1, 1]);
        tessellator.begin_quads(0);
        let mut open = OpenBuild::new(tessellator, "stratum_ghost_winding");
        let (mut out, mut inward) = (0u32, 0u32);
        if let Some(block_tessellator) = self.blocks.as_mut() {
            // SAFETY: the stone block of the joined world and its live region
            unsafe { block_tessellator.tessellate(open.tessellator(), blocks.stone(), pos) };
            let centre = [pos.x as f32 + 0.5, pos.y as f32 + 0.5, pos.z as f32 + 0.5];
            let positions = GameVec::as_mut_slice(&mut open.tessellator().mesh_data().positions);
            for quad in positions.as_chunks::<{ Self::QUAD }>().0 {
                let normal = Self::cross(quad);
                let face: [f32; 3] = std::array::from_fn(|axis| {
                    quad.iter().map(|vertex| vertex[axis]).sum::<f32>() * 0.25 - centre[axis]
                });
                let dot: f32 = (0..3).map(|axis| normal[axis] * face[axis]).sum();
                if dot > 0.0 {
                    out += 1;
                }
                if dot < 0.0 {
                    inward += 1;
                }
            }
        }
        open.discard();
        self.winding = if inward > out { -1.0 } else { 1.0 };
        curia::info!("ghost winding", { cross_points_out: out, cross_points_in: inward, winding: self.winding });
    }

    // the vertices from `first` on belong to one block. The tessellator leaves the cache path's tinted blocks grey,
    // so a block with a tint method whose every vertex is grey has its RGB multiplied by the game's tessellation
    // policy colour at its world position; a full grass block takes it on its up-facing quads only. A block the
    // tessellator did tint is left as it is
    #[allow(unsafe_code, clippy::too_many_arguments)]
    unsafe fn tint(
        game: &BlockCalls,
        data: &mut MeshDataLayout,
        first: usize,
        block: *const c_void,
        region: *mut c_void,
        pos: BlockPos,
        grass_type: *const c_void,
        winding: f32,
    ) {
        // SAFETY: forwarded to the caller: a live block of the world, and its block type
        let (block_type, method) = unsafe {
            let block_type = game.block_type(block);
            (block_type, game.tint_method(block_type))
        };
        if method == 0 {
            return;
        }
        let last = data.positions.len();
        if data.colors.len() != last || last <= first {
            return;
        }
        let grey = data.colors.as_slice()[first..].iter().all(|color| {
            let [r, g, b, _] = color.to_le_bytes();
            r == g && r == b
        });
        if !grey {
            return;
        }
        let top = !grass_type.is_null() && block_type == grass_type;
        if top && !(last - first).is_multiple_of(Self::QUAD) {
            return;
        }
        // SAFETY: forwarded to the caller
        let Some(colour) = (unsafe { game.tint_colour(method, block, region, pos) }) else { return };
        let multiply = [colour[0], colour[1], colour[2]].map(|channel| channel.clamp(0.0, 1.0));
        let positions = &data.positions.as_slice()[first..];
        let colors = &mut GameVec::as_mut_slice(&mut data.colors)[first..];
        for (quad, colors) in colors.chunks_mut(Self::QUAD).enumerate() {
            if top {
                let corners = &positions[quad * Self::QUAD..quad * Self::QUAD + Self::QUAD];
                let normal = Self::cross(corners).map(|axis| axis * winding);
                let length = Self::length(normal);
                if length < Self::MIN_NORMAL || normal[1] < Self::UP_COS * length {
                    continue;
                }
            }
            for color in colors {
                let bytes = color.to_le_bytes();
                let mut out = [0u8, 0, 0, bytes[3]];
                for channel in 0..3 {
                    out[channel] = (f32::from(bytes[channel]) * multiply[channel]).round().min(255.0) as u8;
                }
                *color = u32::from_le_bytes(out);
            }
        }
    }

    // the vertices from `first` on belong to one block: each quad pushed inward along its normal, then the alpha. A
    // block whose vertices are not whole quads keeps its geometry
    fn post(data: &mut MeshDataLayout, first: usize, alpha: u8, winding: f32) {
        let last = data.positions.len();
        if (last - first).is_multiple_of(Self::QUAD) {
            for quad in GameVec::as_mut_slice(&mut data.positions)[first..].as_chunks_mut::<{ Self::QUAD }>().0 {
                let normal = Self::cross(quad);
                let length = Self::length(normal);
                if length < Self::MIN_NORMAL {
                    continue;
                }
                let shift = normal.map(|axis| axis * (-winding * Self::PUSH / length));
                for vertex in quad.iter_mut() {
                    for axis in 0..3 {
                        vertex[axis] += shift[axis];
                    }
                }
            }
        }
        if data.colors.len() != last {
            return;
        }
        // the alpha sits in the high byte of the packed ABGR colour
        let value = u32::from(alpha) << 24;
        for color in &mut GameVec::as_mut_slice(&mut data.colors)[first..] {
            *color = (*color & 0x00FF_FFFF) | value;
        }
    }

    // the box of the part's positions, which the group merge reads
    fn fit_aabb(data: &mut MeshDataLayout) {
        let positions = data.positions.as_slice();
        let Some(first) = positions.first() else {
            data.aabb = [[0.0; 3]; 2];
            return;
        };
        let (mut lo, mut hi) = (*first, *first);
        for vertex in positions {
            for axis in 0..3 {
                lo[axis] = lo[axis].min(vertex[axis]);
                hi[axis] = hi[axis].max(vertex[axis]);
            }
        }
        data.aabb = [lo, hi];
    }

    // the cross product of a quad's first two edges
    fn cross(quad: &[[f32; 3]]) -> [f32; 3] {
        let a: [f32; 3] = std::array::from_fn(|axis| quad[1][axis] - quad[0][axis]);
        let b: [f32; 3] = std::array::from_fn(|axis| quad[2][axis] - quad[0][axis]);
        [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
    }

    fn length(vector: [f32; 3]) -> f32 {
        (vector[0] * vector[0] + vector[1] * vector[1] + vector[2] * vector[2]).sqrt()
    }
}
