use std::ffi::c_void;
use std::ptr::NonNull;

use amml_game::BlockPos;

use crate::game::{BlockTessellatorCacheLayout, BlockTessellatorLayout, Tessellator};
use crate::types::GhostCalls;

// BlockTessellator::BlockTessellator(BlockSource* region), `this` back in RAX
type CtorFn = unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void;
// BlockTessellator::~BlockTessellator()
type DtorFn = unsafe extern "C" fn(*mut c_void);
// void BlockTessellator::setRegion(BlockSource& region)
type SetRegionFn = unsafe extern "C" fn(*mut c_void, *mut c_void);
// bool BlockTessellator::tessellateInWorld(Tessellator&, Block const&, BlockPos const&, bool useCalcWithCache)
type TessellateFn = unsafe extern "C" fn(*mut c_void, *mut c_void, *const c_void, *const BlockPos, u8) -> u8;
// Block const& BlockTessellatorCache::getBlock(BlockPos const&): the reference comes back in RAX
type CacheGetBlockFn = unsafe extern "C" fn(*mut BlockTessellatorCacheLayout, *const BlockPos) -> *const c_void;

/// A `BlockTessellator` the mod owns: allocated on the game heap, built with the game's constructor, and destroyed
/// through `BlockTessellator::~BlockTessellator` and the game's `operator delete` on drop. It keeps a region pointer:
/// before the drop, point it at a live region with `set_region`, or drop it while its region is still live, so the
/// destructor reads no freed `BlockSource`. Render thread only.
pub struct GameBlockTessellator {
    ptr: NonNull<BlockTessellatorLayout>,
    calls: GhostCalls,
}

impl GameBlockTessellator {
    /// Allocates and constructs a tessellator for `region`, with ambient occlusion, normals and the cached block
    /// window off, so `tessellate` reads the neighbours from the region. `None` when the game heap gives no block.
    ///
    /// # Safety
    /// `region` must be a live `BlockSource` of the build `calls` was resolved for, and `calls.heap_new` and
    /// `calls.heap_delete` the game's allocator.
    #[allow(unsafe_code)]
    pub unsafe fn new(region: *mut c_void, calls: &GhostCalls) -> Option<Self> {
        // SAFETY: the caller guarantees the game's allocator; it takes any size
        let block = unsafe { (calls.heap_new)(size_of::<BlockTessellatorLayout>()) };
        let ptr = NonNull::new(block.cast::<BlockTessellatorLayout>())?;
        // SAFETY: the entry is the address of the constructor with the signature of `CtorFn`
        let ctor: CtorFn = unsafe { std::mem::transmute(calls.block_tessellator_ctor) };
        // SAFETY: a block of sizeof(BlockTessellator) from the game heap, constructed here once with a live region
        unsafe { ctor(ptr.as_ptr().cast(), region) };
        // SAFETY: the constructed object is laid out as the mirror, whose anchor entry the calls checked
        unsafe {
            let layout = &mut *ptr.as_ptr();
            layout.apply_ambient_occlusion = false;
            layout.use_normals = false;
            layout.use_cached_calculation = false;
        }
        Some(Self { ptr, calls: *calls })
    }

    /// Points the tessellator and its block window at `region`.
    ///
    /// # Safety
    /// `region` must be a live `BlockSource` of the build the calls were resolved for.
    #[allow(unsafe_code)]
    pub unsafe fn set_region(&mut self, region: *mut c_void) {
        // SAFETY: the entry is the address of `setRegion` with the signature of `SetRegionFn`
        let set_region: SetRegionFn = unsafe { std::mem::transmute(self.calls.block_tessellator_set_region) };
        // SAFETY: a live tessellator; the caller passes a live region
        unsafe { set_region(self.ptr.as_ptr().cast(), region) };
    }

    /// Sets `mUseCachedCalculation`: on, `tessellate` reads the neighbours from the block cache window the caller
    /// fills; off, from the region.
    #[allow(unsafe_code)]
    pub fn set_cached_calculation(&mut self, on: bool) {
        // SAFETY: a live tessellator laid out as the mirror
        unsafe { (*self.ptr.as_ptr()).use_cached_calculation = on };
    }

    /// Sets `mRenderingLayer`, the render layer the next `tessellate` writes for.
    #[allow(unsafe_code)]
    pub fn set_rendering_layer(&mut self, layer: i32) {
        // SAFETY: a live tessellator laid out as the mirror
        unsafe { (*self.ptr.as_ptr()).rendering_layer = layer };
    }

    /// The block cache window, `mBlockCache`, which the caller fills before a cached `tessellate`.
    #[allow(unsafe_code)]
    pub fn cache(&mut self) -> &mut BlockTessellatorCacheLayout {
        // SAFETY: a live tessellator laid out as the mirror, whose anchor entry the calls checked
        unsafe { &mut (*self.ptr.as_ptr()).block_cache }
    }

    /// The block the game's `BlockTessellatorCache::getBlock` reads from the window for `pos` (a `Block const*`).
    #[allow(unsafe_code)]
    pub fn cache_block(&mut self, pos: BlockPos) -> *const c_void {
        // SAFETY: the entry is the address of `BlockTessellatorCache::getBlock` with the signature of
        // `CacheGetBlockFn`
        let get: CacheGetBlockFn = unsafe { std::mem::transmute(self.calls.block_cache_get_block) };
        // SAFETY: the live cache of this tessellator and a position that lives for the call
        unsafe { get(self.cache(), &pos) }
    }

    /// Tessellates `block` (a `Block const*`) at the world position `pos` into `tessellator`, inside the begin the
    /// caller opened. False, with no call, when no begin is open or `block` is null; else what the game returns.
    ///
    /// # Safety
    /// `block` must be null or a live `Block` of the world the tessellator's region belongs to, and that region must
    /// be live.
    #[allow(unsafe_code)]
    pub unsafe fn tessellate(&mut self, tessellator: &mut Tessellator, block: *const c_void, pos: BlockPos) -> bool {
        if block.is_null() || !tessellator.is_tessellating() {
            return false;
        }
        // SAFETY: a live tessellator laid out as the mirror
        let cached = unsafe { (*self.ptr.as_ptr()).use_cached_calculation };
        // SAFETY: the entry is the address of `tessellateInWorld` with the signature of `TessellateFn`
        let tessellate: TessellateFn = unsafe { std::mem::transmute(self.calls.block_tessellator_tessellate_in_world) };
        // SAFETY: a live block tessellator, the live game tessellator with an open begin, a block of the world the
        // region belongs to and a position that lives for the call
        unsafe { tessellate(self.ptr.as_ptr().cast(), tessellator.as_ptr(), block, &pos, u8::from(cached)) != 0 }
    }
}

impl Drop for GameBlockTessellator {
    #[allow(unsafe_code)]
    fn drop(&mut self) {
        // SAFETY: the entry is the address of the destructor with the signature of `DtorFn`
        let dtor: DtorFn = unsafe { std::mem::transmute(self.calls.block_tessellator_dtor) };
        // SAFETY: the object was constructed in `new` and is destroyed here once, then its game heap block, of the
        // size it was allocated with, is freed once
        unsafe {
            dtor(self.ptr.as_ptr().cast());
            (self.calls.heap_delete)(self.ptr.as_ptr().cast(), size_of::<BlockTessellatorLayout>());
        }
    }
}
