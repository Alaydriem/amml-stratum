use std::alloc::{Layout, alloc_zeroed, dealloc};
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Mutex, OnceLock};

use alaydriem_stratum::game::{
    BlockTessellatorCacheLayout, MatrixStackRefLayout, MeshDataLayout, MeshLayout2651, StringByteInputLayout,
    TessellatorLayout, TextureVariant,
};
use alaydriem_stratum::types::GhostCalls;
use amml_game::{BlockPos, HashedStringLayout, StopLatch};
use amml_msvc::MsvcHeap;

use crate::support::FakeBuffer;

/// Fake game functions for the ghost wrappers, with counters. Each test runs in its own process (nextest), so the
/// counters start at zero.
pub struct FakeGhost;

/// The first qword a fake `Tessellator::end` writes into the mesh; the second is the mesh's serial, the count of
/// ends so far.
pub const MESH_MARK: u64 = 0x4D45_5348;
pub const KNOWN_NAME: &str = "minecraft:oak_stairs";
/// The fake tag size the game allocates in `NbtIo::read`.
pub const TAG_BYTES: usize = 0x18;
/// The render layer the fake block graphics hold.
pub const RENDER_LAYER: u8 = 3;
/// Where the fake `BlockType` holds `mTintMethod`.
pub const TINT_METHOD: u64 = 0x17C;
/// The vertices the fake vertex pool holds; a test build stays far below.
const POOL_VERTICES: usize = 1 << 14;

pub static BEGIN_CALLS: AtomicU32 = AtomicU32::new(0);
pub static END_CALLS: AtomicU32 = AtomicU32::new(0);
pub static CLEAR_CALLS: AtomicU32 = AtomicU32::new(0);
/// Clears that closed an open begin.
pub static CLEARS_CLOSED: AtomicU32 = AtomicU32::new(0);
/// A fake `Tessellator::clear` that leaves the begin open, so the caller must end it.
pub static CLEAR_KEEPS_BEGIN: AtomicBool = AtomicBool::new(false);
pub static MESH_DTOR_CALLS: AtomicU32 = AtomicU32::new(0);
pub static MESH_DATA_CLEAR_CALLS: AtomicU32 = AtomicU32::new(0);
/// Whether the mesh data was all zero bytes when the fake `MeshData::clear` ran.
pub static MESH_DATA_CLEARED_ZEROED: AtomicBool = AtomicBool::new(false);
/// The serial of each mesh the fake dtor destroyed, in order.
pub static DESTROYED: Mutex<Vec<u64>> = Mutex::new(Vec::new());
/// Meshes a fake end made that the fake dtor has not destroyed yet.
pub static MESH_LIVE: AtomicU32 = AtomicU32::new(0);
pub static RENDER_CALLS: AtomicU32 = AtomicU32::new(0);
/// The serial of each mesh the fake render drew, in draw order.
pub static RENDERED: Mutex<Vec<u64>> = Mutex::new(Vec::new());
pub static BT_CTOR_CALLS: AtomicU32 = AtomicU32::new(0);
pub static BT_DTOR_CALLS: AtomicU32 = AtomicU32::new(0);
pub static BT_REGION: AtomicUsize = AtomicUsize::new(0);
/// Each `setRegion` (with the region) and each destructor (with the tessellator's region then) in call order.
pub static BT_EVENTS: Mutex<Vec<(&'static str, usize)>> = Mutex::new(Vec::new());
pub static TESSELLATE_CALLS: AtomicU32 = AtomicU32::new(0);
pub static TESSELLATE_OK: AtomicBool = AtomicBool::new(true);
/// The fake `tessellateInWorld` fails on this call (1-based), part way through its quad: it writes 3 of the 4
/// vertices and returns false; 0 never.
pub static TESSELLATE_FAIL_AT: AtomicU32 = AtomicU32::new(0);
/// The fake `tessellateInWorld` raises `LATCH` on this call (1-based); 0 never.
pub static TESSELLATE_RAISE_AT: AtomicU32 = AtomicU32::new(0);
/// The latch a fake raises, as a disable does while a frame runs.
pub static LATCH: StopLatch = StopLatch::new();
pub static LOOKUP_CALLS: AtomicU32 = AtomicU32::new(0);
pub static NBT_READ_CALLS: AtomicU32 = AtomicU32::new(0);
pub static NBT_READ_NULL: AtomicBool = AtomicBool::new(false);
/// What the fake `tryGetBlockFromNBT` answers: 2 Success with `known_block`, else that state and null.
pub static NBT_STATE: AtomicU32 = AtomicU32::new(2);
/// What the fake `tryGetBlockFromNBT` leaves in the 4 bytes after the `NBTState` int, which the game never writes.
pub static NBT_STATE_HIGH: AtomicU32 = AtomicU32::new(0);
pub static TAG_DTOR_CALLS: AtomicU32 = AtomicU32::new(0);
pub static TAGS_LIVE: AtomicU32 = AtomicU32::new(0);
pub static HEAP_LIVE: AtomicU32 = AtomicU32::new(0);
pub static HEAP_NEW_CALLS: AtomicU32 = AtomicU32::new(0);
pub static TAG_DELETES: AtomicU32 = AtomicU32::new(0);
/// Every block the fake heap freed, by address.
pub static HEAP_FREED: Mutex<Vec<usize>> = Mutex::new(Vec::new());
pub static MESH_DATA_DTOR_CALLS: AtomicU32 = AtomicU32::new(0);
pub static MOVE_ASSIGN_CALLS: AtomicU32 = AtomicU32::new(0);
pub static MATRIX_POPS: AtomicU32 = AtomicU32::new(0);

// the fake blocks by name, each with its own fake block type, made once per process
static BLOCKS: OnceLock<Vec<(&'static str, usize)>> = OnceLock::new();
static GRAPHICS: OnceLock<usize> = OnceLock::new();
static MATRIX: OnceLock<usize> = OnceLock::new();
// the fake tessellator's own vertex storage: positions, then colours
static POOL: OnceLock<(usize, usize)> = OnceLock::new();

#[allow(unsafe_code)]
impl FakeGhost {
    /// The names the fake registry knows; `stratum:exact` is the block an exact NBT lookup gives, and no name lookup
    /// gives it.
    pub const NAMES: [&'static str; 6] =
        ["minecraft:air", "minecraft:stone", "minecraft:unknown", "minecraft:grass_block", KNOWN_NAME, "stratum:exact"];

    /// Calls that point at the fakes below.
    pub fn calls() -> GhostCalls {
        GhostCalls {
            tessellator_begin: Self::begin as *const () as usize,
            tessellator_end: Self::end as *const () as usize,
            tessellator_clear: Self::clear as *const () as usize,
            mesh_dtor: Self::mesh_dtor as *const () as usize,
            mesh_is_valid: Self::mesh_is_valid as *const () as usize,
            mesh_render: Self::mesh_render as *const () as usize,
            mesh_data_clear: Self::mesh_data_clear as *const () as usize,
            mesh_data_dtor: Self::mesh_data_dtor as *const () as usize,
            mesh_data_move_assign: Self::mesh_data_move_assign as *const () as usize,
            block_tessellator_ctor: Self::bt_ctor as *const () as usize,
            block_tessellator_dtor: Self::bt_dtor as *const () as usize,
            block_tessellator_set_region: Self::bt_set_region as *const () as usize,
            block_tessellator_tessellate_in_world: Self::bt_tessellate as *const () as usize,
            block_cache_get_block: Self::cache_get_block as *const () as usize,
            block_source_get_block: Self::source_get_block as *const () as usize,
            block_graphics_get_for_block: Self::graphics_for as *const () as usize,
            transform_block: Self::transform as *const () as usize,
            has_transformable_wall_states: Self::has_none as *const () as usize,
            transform_wall_connection_types: Self::transform as *const () as usize,
            has_transformable_connection_states: Self::has_none as *const () as usize,
            transform_connection_states: Self::transform as *const () as usize,
            get_tessellation_policy: 0,
            block_type_registry: 0x5EED,
            lookup_by_name: Self::lookup_by_name as *const () as usize,
            string_byte_input_vtable: 0x7AB1E,
            nbt_read: Self::nbt_read as *const () as usize,
            try_get_block_from_nbt: Self::try_get_block as *const () as usize,
            compound_tag_dtor: Self::tag_dtor as *const () as usize,
            set_entity_constants: 0,
            matrix_stack_push: Self::matrix_push as *const () as usize,
            matrix_stack_ref_dtor: Self::matrix_pop as *const () as usize,
            entity_constant_buffer: 0xC0,
            screen_tessellator: 0xB8,
            atlas_texture: 0x1C0,
            frame_views: 0x10,
            item_in_hand_renderer_slot: 353,
            blend_material: 0x1D8,
            render_layer: 0x1A,
            tessellation_policy_get_slot: 2,
            tint_method: TINT_METHOD,
            heap_new: Self::heap_new,
            heap_delete: Self::heap_delete,
        }
    }

    /// The fake heap as the MSVC STL uses it.
    pub fn heap() -> MsvcHeap {
        MsvcHeap::new(Self::heap_new, Self::heap_delete)
    }

    /// A zeroed block laid out as the game's `Tessellator`, up to `mTessellating`.
    pub fn tessellator() -> FakeBuffer {
        FakeBuffer::new(size_of::<TessellatorLayout>())
    }

    pub fn count(counter: &AtomicU32) -> u32 {
        counter.load(Ordering::SeqCst)
    }

    /// The fake block named `name` (one of `NAMES`), a `Block` whose `mBlockType` points at its own fake type; 0
    /// for any other name.
    pub fn block(name: &str) -> usize {
        let blocks = BLOCKS.get_or_init(|| {
            Self::NAMES
                .iter()
                .map(|name| {
                    let block_type = Box::leak(vec![0u64; 0x200 / 8].into_boxed_slice()).as_ptr() as u64;
                    let mut block = vec![0u64; 0x80 / 8];
                    block[0x68 / 8] = block_type;
                    (*name, Box::leak(block.into_boxed_slice()).as_ptr() as usize)
                })
                .collect()
        });
        blocks.iter().find(|(known, _)| *known == name).map_or(0, |(_, block)| *block)
    }

    /// The block an exact NBT lookup gives.
    pub fn known_block() -> usize {
        Self::block("stratum:exact")
    }

    /// The block a lookup by `KNOWN_NAME` with the default states gives.
    pub fn name_only_block() -> usize {
        Self::block(KNOWN_NAME)
    }

    /// The fake block graphics every block has.
    pub fn graphics() -> usize {
        *GRAPHICS.get_or_init(|| {
            let mut bytes = vec![0u8; 0x40];
            bytes[0x1A] = RENDER_LAYER;
            Box::leak(bytes.into_boxed_slice()).as_ptr() as usize
        })
    }

    fn pool() -> (usize, usize) {
        *POOL.get_or_init(|| {
            let positions = Box::leak(vec![[0f32; 3]; POOL_VERTICES].into_boxed_slice()).as_mut_ptr() as usize;
            let colors = Box::leak(vec![0u32; POOL_VERTICES].into_boxed_slice()).as_mut_ptr() as usize;
            (positions, colors)
        })
    }

    unsafe fn data<'a>(this: *mut c_void) -> &'a mut MeshDataLayout {
        // SAFETY: the tests pass a fake tessellator laid out as the mirror
        unsafe { &mut (*this.cast::<TessellatorLayout>()).mesh_data }
    }

    unsafe fn set_tessellating(this: *mut c_void, value: bool) {
        // SAFETY: the tests pass a fake tessellator laid out as the mirror
        unsafe { (*this.cast::<TessellatorLayout>()).tessellating = value };
    }

    // points the positions and colours at the empty pool
    unsafe fn empty_pool(data: &mut MeshDataLayout) {
        let (positions, colors) = Self::pool();
        let positions = positions as *mut [f32; 3];
        let colors = colors as *mut u32;
        // SAFETY: the pool holds POOL_VERTICES elements of each kind
        unsafe {
            data.positions.set_raw_parts(positions, positions, positions.add(POOL_VERTICES));
            data.colors.set_raw_parts(colors, colors, colors.add(POOL_VERTICES));
        }
    }

    // frees, through the fake heap, every vector of `data` that is not the pool
    unsafe fn release_all(data: &mut MeshDataLayout) {
        let heap = Self::heap();
        let (pool, _) = Self::pool();
        // SAFETY: every vector that is not the pool came from the fake heap
        unsafe {
            if data.positions.raw_parts()[0] as usize != pool {
                data.positions.release(&heap);
                data.colors.release(&heap);
            }
            data.normals.release(&heap);
            data.tangents.release(&heap);
            data.indices.release(&heap);
            data.bone_id0s.release(&heap);
            for uv in &mut data.texture_uvs {
                uv.release(&heap);
            }
            data.pbr_texture_indices.release(&heap);
            data.mers.release(&heap);
            data.geo_type.release(&heap);
        }
    }

    unsafe extern "C" fn begin(this: *mut c_void, _callback: u8, mode: u8, _reserved: i32, _flag: u8) {
        BEGIN_CALLS.fetch_add(1, Ordering::SeqCst);
        // SAFETY: as in `set_tessellating`
        unsafe {
            Self::set_tessellating(this, true);
            let data = Self::data(this);
            Self::release_all(data);
            Self::empty_pool(data);
            data.mode = mode;
        }
    }

    unsafe extern "C" fn end(
        this: *mut c_void,
        out: *mut MeshLayout2651,
        _upload: i32,
        _name: *const [usize; 2],
        _mode: u16,
    ) -> *mut MeshLayout2651 {
        let serial = END_CALLS.fetch_add(1, Ordering::SeqCst) + 1;
        MESH_LIVE.fetch_add(1, Ordering::SeqCst);
        // SAFETY: as in `set_tessellating`; `out` is the 632-byte mesh buffer the wrapper passes
        unsafe {
            Self::set_tessellating(this, false);
            let data = Self::data(this);
            Self::release_all(data);
            Self::empty_pool(data);
            out.cast::<u8>().write_bytes(0, size_of::<MeshLayout2651>());
            out.cast::<u64>().write(MESH_MARK);
            out.cast::<u64>().add(1).write(u64::from(serial));
        }
        out
    }

    unsafe extern "C" fn clear(this: *mut c_void) {
        CLEAR_CALLS.fetch_add(1, Ordering::SeqCst);
        // SAFETY: as in `set_tessellating`
        unsafe {
            let data = Self::data(this);
            Self::release_all(data);
            Self::empty_pool(data);
            if !CLEAR_KEEPS_BEGIN.load(Ordering::SeqCst) {
                if (*this.cast::<TessellatorLayout>()).tessellating {
                    CLEARS_CLOSED.fetch_add(1, Ordering::SeqCst);
                }
                Self::set_tessellating(this, false);
            }
        }
    }

    unsafe extern "C" fn mesh_dtor(mesh: *mut MeshLayout2651) {
        // SAFETY: the wrapper passes the mesh the fake end wrote
        let mark = unsafe { mesh.cast::<u64>().read() };
        assert_eq!(mark, MESH_MARK, "the dtor got a mesh the fake end did not write");
        MESH_DTOR_CALLS.fetch_add(1, Ordering::SeqCst);
        MESH_LIVE.fetch_sub(1, Ordering::SeqCst);
        // SAFETY: as above
        DESTROYED.lock().unwrap().push(unsafe { mesh.cast::<u64>().add(1).read() });
    }

    unsafe extern "C" fn mesh_is_valid(mesh: *const MeshLayout2651) -> u8 {
        // SAFETY: as in `mesh_dtor`
        u8::from(unsafe { mesh.cast::<u64>().read() } == MESH_MARK)
    }

    unsafe extern "C" fn mesh_render(
        mesh: *const MeshLayout2651,
        _context: *mut c_void,
        _material: *const c_void,
        _texture: *const TextureVariant,
        _start: u32,
        _count: u32,
        _capture: *const c_void,
        _indices: *const c_void,
    ) {
        RENDER_CALLS.fetch_add(1, Ordering::SeqCst);
        // SAFETY: as in `mesh_dtor`
        let serial = unsafe { mesh.cast::<u64>().add(1).read() };
        RENDERED.lock().unwrap().push(serial);
    }

    unsafe extern "C" fn mesh_data_clear(this: *mut MeshDataLayout) {
        MESH_DATA_CLEAR_CALLS.fetch_add(1, Ordering::SeqCst);
        // SAFETY: the wrapper passes a MeshData-sized buffer
        let bytes = unsafe { std::slice::from_raw_parts(this.cast::<u8>(), size_of::<MeshDataLayout>()) };
        MESH_DATA_CLEARED_ZEROED.store(bytes.iter().all(|byte| *byte == 0), Ordering::SeqCst);
    }

    unsafe extern "C" fn mesh_data_dtor(this: *mut MeshDataLayout) {
        MESH_DATA_DTOR_CALLS.fetch_add(1, Ordering::SeqCst);
        // SAFETY: a mesh data the fake ctor made, whose vectors came from the fake heap
        unsafe { Self::release_all(&mut *this) };
    }

    unsafe extern "C" fn mesh_data_move_assign(
        this: *mut MeshDataLayout,
        from: *mut MeshDataLayout,
    ) -> *mut MeshDataLayout {
        MOVE_ASSIGN_CALLS.fetch_add(1, Ordering::SeqCst);
        // SAFETY: two live mesh data; the source's vectors are taken and it is left empty
        unsafe {
            Self::release_all(&mut *this);
            std::ptr::copy_nonoverlapping(from, this, 1);
            from.cast::<u8>().write_bytes(0, size_of::<MeshDataLayout>());
        }
        this
    }

    unsafe extern "C" fn bt_ctor(this: *mut c_void, region: *mut c_void) -> *mut c_void {
        BT_CTOR_CALLS.fetch_add(1, Ordering::SeqCst);
        BT_REGION.store(region as usize, Ordering::SeqCst);
        // SAFETY: the wrapper passes a BlockTessellator-sized block; the region sits at +8
        unsafe { this.cast::<usize>().add(1).write(region as usize) };
        this
    }

    unsafe extern "C" fn bt_dtor(this: *mut c_void) {
        BT_DTOR_CALLS.fetch_add(1, Ordering::SeqCst);
        // SAFETY: as in `bt_ctor`
        let region = unsafe { this.cast::<usize>().add(1).read() };
        BT_EVENTS.lock().unwrap().push(("dtor", region));
    }

    unsafe extern "C" fn bt_set_region(this: *mut c_void, region: *mut c_void) {
        BT_REGION.store(region as usize, Ordering::SeqCst);
        BT_EVENTS.lock().unwrap().push(("set_region", region as usize));
        // SAFETY: as in `bt_ctor`
        unsafe { this.cast::<usize>().add(1).write(region as usize) };
    }

    // writes the top face of the block at `pos` as one grey quad, as the game appends a block's quads
    unsafe extern "C" fn bt_tessellate(
        _this: *mut c_void,
        tessellator: *mut c_void,
        _block: *const c_void,
        pos: *const BlockPos,
        _cached: u8,
    ) -> u8 {
        let call = TESSELLATE_CALLS.fetch_add(1, Ordering::SeqCst) + 1;
        // SAFETY: as in `set_tessellating`
        let tessellating = unsafe { (*tessellator.cast::<TessellatorLayout>()).tessellating };
        assert!(tessellating, "tessellateInWorld ran outside a begin and end");
        if TESSELLATE_RAISE_AT.load(Ordering::SeqCst) == call {
            LATCH.raise();
        }
        let failed = TESSELLATE_FAIL_AT.load(Ordering::SeqCst) == call;
        // SAFETY: the wrapper passes a live position; a begin pointed the positions and colours at the pool
        unsafe {
            let pos = *pos;
            let data = Self::data(tessellator);
            let [x, y, z] = [pos.x as f32, pos.y as f32 + 1.0, pos.z as f32];
            let quad = [[x, y, z], [x, y, z + 1.0], [x + 1.0, y, z + 1.0], [x + 1.0, y, z]];
            let written = if failed { 3 } else { 4 };
            for vertex in quad.into_iter().take(written) {
                let [first, last, end] = data.positions.raw_parts();
                assert!(last < end, "the fake pool is full");
                last.write(vertex);
                data.positions.set_raw_parts(first, last.add(1), end);
                let [first, last, end] = data.colors.raw_parts();
                last.write(0xFF80_8080);
                data.colors.set_raw_parts(first, last.add(1), end);
            }
        }
        u8::from(TESSELLATE_OK.load(Ordering::SeqCst) && !failed)
    }

    // the window holds the axis order x, y, z, with mPos at the window's corner
    unsafe extern "C" fn cache_get_block(
        this: *mut BlockTessellatorCacheLayout,
        pos: *const BlockPos,
    ) -> *const c_void {
        // SAFETY: the wrapper passes the cache of a live tessellator and a live position
        unsafe {
            let cache = &*this;
            let pos = *pos;
            let d = [pos.x - cache.pos[0], pos.y - cache.pos[1], pos.z - cache.pos[2]];
            if d.iter().any(|axis| !(0..20).contains(axis)) {
                return cache.air_block;
            }
            cache.blocks[((d[0] * 20 + d[1]) * 20 + d[2]) as usize]
        }
    }

    // the world holds air everywhere
    unsafe extern "C" fn source_get_block(_this: *mut c_void, _pos: *const BlockPos) -> *const c_void {
        Self::block("minecraft:air") as *const c_void
    }

    unsafe extern "C" fn graphics_for(_block: *const c_void) -> *const c_void {
        Self::graphics() as *const c_void
    }

    unsafe extern "C" fn transform(block: *const c_void, _rotation: u8, _mirror: u8) -> *const c_void {
        block
    }

    unsafe extern "C" fn has_none(_block: *const c_void) -> u8 {
        0
    }

    unsafe extern "C" fn matrix_push(stack: *mut c_void, out: *mut MatrixStackRefLayout, _sort: u8) -> *mut c_void {
        let matrix = *MATRIX.get_or_init(|| {
            let identity = [[1.0f32, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];
            Box::leak(Box::new(identity)) as *mut [[f32; 4]; 4] as usize
        });
        // SAFETY: the wrapper passes the 16-byte ref buffer
        unsafe { out.write(MatrixStackRefLayout { stack, matrix: matrix as *mut [[f32; 4]; 4] }) };
        out.cast()
    }

    unsafe extern "C" fn matrix_pop(_reference: *mut MatrixStackRefLayout) {
        MATRIX_POPS.fetch_add(1, Ordering::SeqCst);
    }

    unsafe extern "C" fn lookup_by_name(
        _this: *const c_void,
        out: *mut [usize; 2],
        name: *const HashedStringLayout,
        _data: i32,
        _resolve: i32,
        _log: u8,
    ) -> *mut [usize; 2] {
        LOOKUP_CALLS.fetch_add(1, Ordering::SeqCst);
        // SAFETY: the wrapper passes a HashedString it built for the call
        let text = unsafe { (*name).text.to_str().map(str::to_string).unwrap_or_default() };
        let block = if text == "stratum:exact" { 0 } else { Self::block(&text) };
        // SAFETY: `out` is the two-pointer result buffer of the call
        unsafe { out.write([0x7E, block]) };
        out
    }

    unsafe extern "C" fn nbt_read(out: *mut *mut c_void, input: *mut StringByteInputLayout) -> *mut *mut c_void {
        NBT_READ_CALLS.fetch_add(1, Ordering::SeqCst);
        // SAFETY: the wrapper passes its input and a pointer-sized result
        unsafe {
            assert_eq!((*input).vtable as usize, 0x7AB1E);
            if NBT_READ_NULL.load(Ordering::SeqCst) {
                out.write(std::ptr::null_mut());
            } else {
                let tag = Self::heap_new(TAG_BYTES).cast::<c_void>();
                TAGS_LIVE.fetch_add(1, Ordering::SeqCst);
                out.write(tag);
            }
        }
        out
    }

    unsafe extern "C" fn try_get_block(out: *mut [u64; 2], tag: *const c_void, _cache: *mut c_void) -> *mut [u64; 2] {
        assert!(!tag.is_null());
        let state = NBT_STATE.load(Ordering::SeqCst);
        let block = if state == 2 { Self::known_block() as u64 } else { 0 };
        let first = u64::from(NBT_STATE_HIGH.load(Ordering::SeqCst)) << 32 | u64::from(state);
        // SAFETY: `out` is the 16-byte pair the wrapper passes
        unsafe { out.write([first, block]) };
        out
    }

    unsafe extern "C" fn tag_dtor(_tag: *mut c_void) {
        TAG_DTOR_CALLS.fetch_add(1, Ordering::SeqCst);
        TAGS_LIVE.fetch_sub(1, Ordering::SeqCst);
    }

    pub unsafe extern "C" fn heap_new(size: usize) -> *mut u8 {
        HEAP_LIVE.fetch_add(1, Ordering::SeqCst);
        HEAP_NEW_CALLS.fetch_add(1, Ordering::SeqCst);
        // SAFETY: a non-zero size with a valid alignment
        unsafe { alloc_zeroed(Layout::from_size_align(size.max(1), 16).unwrap()) }
    }

    pub unsafe extern "C" fn heap_delete(ptr: *mut u8, size: usize) {
        HEAP_LIVE.fetch_sub(1, Ordering::SeqCst);
        if size == TAG_BYTES {
            TAG_DELETES.fetch_add(1, Ordering::SeqCst);
        }
        HEAP_FREED.lock().unwrap().push(ptr as usize);
        // SAFETY: `ptr` came from `heap_new` with the same size
        unsafe { dealloc(ptr, Layout::from_size_align(size.max(1), 16).unwrap()) };
    }
}
