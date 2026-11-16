use std::ffi::c_void;

use amml_game::{BlockLayout, BlockPos, RawObject};

use crate::types::GhostCalls;

// static BlockGraphics* BlockGraphics::getForBlock(Block const&)
type GraphicsFn = unsafe extern "C" fn(*const c_void) -> *const c_void;
// BlockRenderLayer Block::getRenderLayer() const: the one-byte layer comes back in AL
type RenderLayerFn = unsafe extern "C" fn(*const c_void) -> u8;
// int BlockType::getExtraRenderLayers() const: a bit mask of BlockRenderLayer
type ExtraLayersFn = unsafe extern "C" fn(*const c_void) -> u32;
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

// LevelChunk* BlockSource::getChunk(int x, int z) const: a virtual member, the chunk comes back in RAX
type ChunkFn = unsafe extern "C" fn(*mut c_void, i32, i32) -> *mut c_void;
// Biome const& LevelChunk::getBiome(ChunkBlockPos const&) const: the reference comes back in RAX
type BiomeFn = unsafe extern "C" fn(*const c_void, *const u32) -> *const c_void;
// int BiomeColorSampling::getWaterColor(Biome const&, BlockPos const&): the packed ARGB colour comes back in EAX
type WaterColorFn = unsafe extern "C" fn(*const c_void, *const BlockPos) -> u32;

/// The game calls and reads on a `Block` that the ghost palette and the tint need: its graphics and render layers,
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
    /// Where `BlockGraphics` holds `mBlockShape`.
    const BLOCK_SHAPE: u64 = 0x10;

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

    /// The `BlockShape` of a block's graphics (`mBlockShape`); 0, `Block`, for null graphics.
    ///
    /// # Safety
    /// `graphics` must be null or a live `BlockGraphics` of the game.
    #[allow(unsafe_code)]
    pub unsafe fn block_shape(&self, graphics: *const c_void) -> i32 {
        if graphics.is_null() {
            return 0;
        }
        // SAFETY: a live block graphics, holding the four-byte shape at `mBlockShape`
        unsafe { RawObject::read::<i32>(graphics, Self::BLOCK_SHAPE) }.unwrap_or(0)
    }

    /// The main render layer of `block` (`Block::getRenderLayer`), the layer the chunk builder queues it in; 0, with
    /// no call, for a null block.
    ///
    /// # Safety
    /// `block` must be null or a live `Block` of the game.
    #[allow(unsafe_code)]
    pub unsafe fn render_layer(&self, block: *const c_void) -> u8 {
        if block.is_null() {
            return 0;
        }
        // SAFETY: the entry is the address of `Block::getRenderLayer` with the signature of `RenderLayerFn`
        let get: RenderLayerFn = unsafe { std::mem::transmute(self.calls.block_get_render_layer) };
        // SAFETY: a live block of the world
        unsafe { get(block) }
    }

    /// The extra render layers of a block type (`BlockType::getExtraRenderLayers`), one bit per layer; 0 for a null
    /// type or a slot that cannot be read.
    ///
    /// # Safety
    /// `block_type` must be null or a live `BlockType` of the game.
    #[allow(unsafe_code)]
    pub unsafe fn extra_render_layers(&self, block_type: *const c_void) -> u32 {
        if block_type.is_null() {
            return 0;
        }
        // SAFETY: a live block type with a vtable that holds `getExtraRenderLayers` at the entry's slot
        let Some(get) = (unsafe { RawObject::slot(block_type, self.calls.extra_render_layers_slot) }) else {
            return 0;
        };
        // SAFETY: the slot holds the virtual `getExtraRenderLayers` with the signature of `ExtraLayersFn`
        let get: ExtraLayersFn = unsafe { std::mem::transmute(get) };
        // SAFETY: a live block type
        unsafe { get(block_type) }
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

    /// The water colour of the biome at `pos` in `region` as red, green, blue and alpha in 0 to 1: the chunk
    /// (`BlockSource::getChunk`), its biome (`LevelChunk::getBiome`) and the biome's water colour; the alpha is the
    /// colour's own. `None` when the chunk or the biome is null.
    ///
    /// # Safety
    /// `region` must be a live `BlockSource` of the game.
    #[allow(unsafe_code)]
    pub unsafe fn water_colour(&self, region: *mut c_void, pos: BlockPos) -> Option<[f32; 4]> {
        if region.is_null()
            || self.calls.get_chunk_slot == 0
            || self.calls.level_chunk_get_biome == 0
            || self.calls.biome_get_water_color == 0
        {
            return None;
        }
        // SAFETY: a live region with a vtable that holds `getChunk` at the entry's slot
        let get_chunk = unsafe { RawObject::slot(region, self.calls.get_chunk_slot) }?;
        // SAFETY: the slot holds the virtual `getChunk` with the signature of `ChunkFn`
        let get_chunk: ChunkFn = unsafe { std::mem::transmute(get_chunk) };
        // SAFETY: the caller's live region; the arithmetic shift floors negative positions to their chunk
        let chunk = unsafe { get_chunk(region, pos.x >> 4, pos.z >> 4) };
        if chunk.is_null() {
            return None;
        }
        // ChunkBlockPos { u8 x, u8 z, i16 y } as one little-endian int
        let column =
            u32::from((pos.x & 15) as u8) | u32::from((pos.z & 15) as u8) << 8 | u32::from(pos.y as i16 as u16) << 16;
        // SAFETY: the entry is the address of `getBiome` with the signature of `BiomeFn`
        let get_biome: BiomeFn = unsafe { std::mem::transmute(self.calls.level_chunk_get_biome) };
        // SAFETY: the chunk the region gave and a four-byte column that lives for the call
        let biome = unsafe { get_biome(chunk, &column) };
        if biome.is_null() {
            return None;
        }
        // SAFETY: the entry is the address of `getWaterColor` with the signature of `WaterColorFn`
        let get_colour: WaterColorFn = unsafe { std::mem::transmute(self.calls.biome_get_water_color) };
        // SAFETY: the biome the chunk gave and a position that lives for the call
        let argb = unsafe { get_colour(biome, &pos) };
        let [b, g, r, a] = argb.to_le_bytes();
        Some([r, g, b, a].map(|channel| f32::from(channel) / 255.0))
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
