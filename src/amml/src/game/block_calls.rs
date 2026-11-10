use std::ffi::c_void;

use amml_game::{BlockLayout, BlockPos, RawObject};

use crate::types::GhostCalls;

// static BlockGraphics* BlockGraphics::getForBlock(Block const&)
type GraphicsFn = unsafe extern "C" fn(*const c_void) -> *const c_void;
// static Block const* VanillaBlockStateTransformUtils::transform*(Block const&, Rotation, Mirror), both one byte
type TransformFn = unsafe extern "C" fn(*const c_void, u8, u8) -> *const c_void;
// static bool VanillaBlockStateTransformUtils::hasTransformable*States(Block const&)
type HasFn = unsafe extern "C" fn(*const c_void) -> u8;
// Block const& BlockSource::getBlock(BlockPos const&) const: the reference comes back in RAX
type WorldBlockFn = unsafe extern "C" fn(*mut c_void, *const BlockPos) -> *const c_void;
// TessellationPolicy const& BiomeColorSampling::getTessellationPolicy(TintMethod)
type PolicyFn = unsafe extern "C" fn(u8) -> *const c_void;
// mce::Color TessellationPolicy::get(Block const&, BlockSource&, BlockPos const&, BiomeTintCache const*) const: a
// virtual member that returns its 16 bytes through the hidden pointer, after `this`
type PolicyGetFn = unsafe extern "C" fn(
    *const c_void,
    *mut [f32; 4],
    *const c_void,
    *mut c_void,
    *const BlockPos,
    *const c_void,
) -> *mut [f32; 4];

/// The game calls and reads on a `Block` that the ghost palette and the tint need: its graphics and render layer,
/// its turn, its block type and tint method, the biome colour of a tint, and the block a region holds. Each block
/// is a `Block const*` of the world it was looked up in.
#[derive(Clone, Copy)]
pub struct BlockCalls {
    calls: GhostCalls,
}

impl BlockCalls {
    /// `Mirror::None`.
    const NO_MIRROR: u8 = 0;
    /// `TintMethod::Size`: the methods the game's policy table holds.
    const TINT_METHODS: u8 = 9;

    pub fn new(calls: &GhostCalls) -> Self {
        Self { calls: *calls }
    }

    /// `BlockGraphics::getForBlock`; null when the game has none or `block` is null.
    ///
    /// # Safety
    /// `block` must be null or a live `Block` of the game.
    #[allow(unsafe_code)]
    pub unsafe fn graphics(&self, block: *const c_void) -> *const c_void {
        if block.is_null() {
            return std::ptr::null();
        }
        // SAFETY: the entry is the address of `getForBlock` with the signature of `GraphicsFn`
        let get: GraphicsFn = unsafe { std::mem::transmute(self.calls.block_graphics_get_for_block) };
        // SAFETY: a live block of the world
        unsafe { get(block) }
    }

    /// `BlockGraphics::mRenderLayer`; 0 for null graphics.
    ///
    /// # Safety
    /// `graphics` must be null or live `BlockGraphics` the game gave.
    #[allow(unsafe_code)]
    pub unsafe fn render_layer(&self, graphics: *const c_void) -> i32 {
        if graphics.is_null() {
            return 0;
        }
        // SAFETY: live graphics the game gave, holding the one-byte layer at the entry's offset
        unsafe { RawObject::read::<u8>(graphics, self.calls.render_layer) }.map_or(0, i32::from)
    }

    /// `block` turned by `quarters` clockwise quarter turns seen from above, as the structure placement turns it: its
    /// states, then its connection states, then its wall states. A transform that gives null keeps the block.
    ///
    /// # Safety
    /// `block` must be null or a live `Block` of the game.
    #[allow(unsafe_code)]
    pub unsafe fn turn(&self, block: *const c_void, quarters: u8) -> *const c_void {
        let rotation = quarters & 3;
        if block.is_null() || rotation == 0 {
            return block;
        }
        // SAFETY: the entries are the addresses of the five functions with the signatures above
        let (transform, has_connection, connection, has_wall, wall): (
            TransformFn,
            HasFn,
            TransformFn,
            HasFn,
            TransformFn,
        ) = unsafe {
            (
                std::mem::transmute::<usize, TransformFn>(self.calls.transform_block),
                std::mem::transmute::<usize, HasFn>(self.calls.has_transformable_connection_states),
                std::mem::transmute::<usize, TransformFn>(self.calls.transform_connection_states),
                std::mem::transmute::<usize, HasFn>(self.calls.has_transformable_wall_states),
                std::mem::transmute::<usize, TransformFn>(self.calls.transform_wall_connection_types),
            )
        };
        let mut block = block;
        // SAFETY: each call gets a live block of the world, a rotation of 1 to 3 and no mirror
        unsafe {
            let turned = transform(block, rotation, Self::NO_MIRROR);
            if !turned.is_null() {
                block = turned;
            }
            if has_connection(block) != 0 {
                let turned = connection(block, rotation, Self::NO_MIRROR);
                if !turned.is_null() {
                    block = turned;
                }
            }
            if has_wall(block) != 0 {
                let turned = wall(block, rotation, Self::NO_MIRROR);
                if !turned.is_null() {
                    block = turned;
                }
            }
        }
        block
    }

    /// The block type (`mBlockType`); null for a null block.
    ///
    /// # Safety
    /// `block` must be null or a live `Block` of the game.
    #[allow(unsafe_code)]
    pub unsafe fn block_type(&self, block: *const c_void) -> *const c_void {
        if block.is_null() {
            return std::ptr::null();
        }
        // SAFETY: a live block, which `BlockLayout` mirrors up to `mBlockType`
        unsafe { (&raw const (*block.cast::<BlockLayout>()).block_type).read_unaligned() }
    }

    /// The tint method of a block type (`mTintMethod`); 0, `TintMethod::None`, for a null type.
    ///
    /// # Safety
    /// `block_type` must be null or a live `BlockType` of the game.
    #[allow(unsafe_code)]
    pub unsafe fn tint_method(&self, block_type: *const c_void) -> u8 {
        if block_type.is_null() {
            return 0;
        }
        // SAFETY: a live block type, holding the one-byte method at the entry's offset
        unsafe { RawObject::read::<u8>(block_type, self.calls.tint_method) }.unwrap_or(0)
    }

    /// The colour the game's tessellation policy for `method` gives `block` at `pos` in `region`, with no tint
    /// cache; `None` for a method outside the policy table or when the game gives no policy.
    ///
    /// # Safety
    /// `block` must be a live block of the world `region` belongs to, and `region` a live `BlockSource`.
    #[allow(unsafe_code)]
    pub unsafe fn tint_colour(
        &self,
        method: u8,
        block: *const c_void,
        region: *mut c_void,
        pos: BlockPos,
    ) -> Option<[f32; 4]> {
        if method == 0 || method >= Self::TINT_METHODS || self.calls.get_tessellation_policy == 0 {
            return None;
        }
        // SAFETY: the entry is the address of `getTessellationPolicy` with the signature of `PolicyFn`
        let policy_of: PolicyFn = unsafe { std::mem::transmute(self.calls.get_tessellation_policy) };
        // SAFETY: a method inside the game's table
        let policy = unsafe { policy_of(method) };
        if policy.is_null() {
            return None;
        }
        // SAFETY: the policy is a live object with a vtable that holds `get` at the entry's slot
        let get = unsafe { RawObject::slot(policy, self.calls.tessellation_policy_get_slot) }?;
        // SAFETY: the slot holds the virtual `get` with the signature of `PolicyGetFn`
        let get: PolicyGetFn = unsafe { std::mem::transmute(get) };
        let mut colour = [0.0f32; 4];
        // SAFETY: the live policy, a 16-byte result buffer, and the caller's live block, region and position
        unsafe { get(policy, &mut colour, block, region, &pos, std::ptr::null()) };
        Some(colour)
    }

    /// The block `region` holds at `pos` on layer 0.
    ///
    /// # Safety
    /// `region` must be a live `BlockSource` of the game.
    #[allow(unsafe_code)]
    pub unsafe fn world_block(&self, region: *mut c_void, pos: BlockPos) -> *const c_void {
        // SAFETY: the entry is the address of `BlockSource::getBlock` with the signature of `WorldBlockFn`
        let get: WorldBlockFn = unsafe { std::mem::transmute(self.calls.block_source_get_block) };
        // SAFETY: the caller's live region and a position that lives for the call
        unsafe { get(region, &pos) }
    }
}
