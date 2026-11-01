use std::mem::offset_of;

use amml_game::{CameraLayout, EntryId, GameEntries};

use crate::errors::GameLayoutError;
use crate::game::{BlockTessellatorLayout, MeshLayout2651, MeshLayoutKind, TessellatorLayout};
use crate::types::StratumEntry;

/// The game's allocator: `operator new(size)`.
pub type HeapNewFn = unsafe extern "C" fn(usize) -> *mut u8;
/// The game's sized `operator delete(ptr, size)`.
pub type HeapDeleteFn = unsafe extern "C" fn(*mut u8, usize);

/// The resolved game code and offsets the ghost wrappers call and read: code addresses, vtable slots and byte
/// offsets, as the entries give them, and the game heap. A test fills it with fake functions.
#[derive(Clone, Copy, Debug)]
pub struct GhostCalls {
    pub tessellator_begin: usize,
    pub tessellator_end: usize,
    pub tessellator_clear: usize,
    pub mesh_dtor: usize,
    pub mesh_is_valid: usize,
    pub mesh_render: usize,
    /// `void mce::MeshData::clear()`, run on zeroed bytes to make an empty MeshData.
    pub mesh_data_clear: usize,
    pub mesh_data_dtor: usize,
    pub mesh_data_move_assign: usize,
    pub block_tessellator_ctor: usize,
    pub block_tessellator_dtor: usize,
    pub block_tessellator_set_region: usize,
    pub block_tessellator_tessellate_in_world: usize,
    pub block_cache_get_block: usize,
    /// `BlockSource::getBlock(BlockPos const&) const`, the body of its vtable slot.
    pub block_source_get_block: usize,
    pub block_graphics_get_for_block: usize,
    pub transform_block: usize,
    pub has_transformable_wall_states: usize,
    pub transform_wall_connection_types: usize,
    pub has_transformable_connection_states: usize,
    pub transform_connection_states: usize,
    pub get_tessellation_policy: usize,
    /// The address of the `BlockTypeRegistry` object.
    pub block_type_registry: usize,
    pub lookup_by_name: usize,
    pub string_byte_input_vtable: usize,
    pub nbt_read: usize,
    pub try_get_block_from_nbt: usize,
    pub compound_tag_dtor: usize,
    pub set_entity_constants: usize,
    pub matrix_stack_push: usize,
    pub matrix_stack_ref_dtor: usize,
    /// Offset of `entityConstantBuffer` in `GlobalConstantBuffers`.
    pub entity_constant_buffer: u64,
    /// Offset of the `Tessellator&` in `ScreenContext`.
    pub screen_tessellator: u64,
    /// Offset of `mAtlasTexture` in `LevelRenderer`.
    pub atlas_texture: u64,
    /// Offset of `mViews` in `FrameRenderObject`.
    pub frame_views: u64,
    /// Vtable slot of `ClientInstance::getItemInHandRenderer`.
    pub item_in_hand_renderer_slot: u64,
    /// Offset of `mMatBlendBlock` in `ItemInHandRenderer`.
    pub blend_material: u64,
    /// Offset of `mRenderLayer` (one byte) in `BlockGraphics`.
    pub render_layer: u64,
    /// Vtable slot of `TessellationPolicy::get(Block const&, BlockSource&, BlockPos const&, BiomeTintCache const*)`.
    pub tessellation_policy_get_slot: u64,
    /// Offset of `mTintMethod` (one byte) in `BlockType`.
    pub tint_method: u64,
    pub heap_new: HeapNewFn,
    pub heap_delete: HeapDeleteFn,
}

impl GhostCalls {
    /// The calls of the build `entries` was resolved for. An error names the first entry that is missing, or the
    /// mirror anchor that does not fit: `Tessellator.mTessellating`, `BlockTessellator.mBlockCache` and
    /// `Camera.mForward` must hold the offsets the mirrors were written with, and `Mesh.size` the size of one of the
    /// mesh mirrors.
    pub fn new(entries: &GameEntries, heap_new: HeapNewFn, heap_delete: HeapDeleteFn) -> Result<Self, GameLayoutError> {
        let get = |id: EntryId| entries.get(id).ok_or(GameLayoutError::Missing { entry: id.as_str() });
        let address = |id: EntryId| get(id).map(|value| value as usize);
        let anchor = |id: EntryId, expected: u64| {
            let found = get(id)?;
            if found == expected {
                Ok(())
            } else {
                Err(GameLayoutError::Misfit { entry: id.as_str(), expected, found })
            }
        };
        anchor(StratumEntry::TESSELLATOR_TESSELLATING, offset_of!(TessellatorLayout, tessellating) as u64)?;
        anchor(StratumEntry::BLOCK_TESSELLATOR_BLOCK_CACHE, offset_of!(BlockTessellatorLayout, block_cache) as u64)?;
        if MeshLayoutKind::of(entries).is_none() {
            let found = get(StratumEntry::MESH_SIZE)?;
            return Err(GameLayoutError::Misfit {
                entry: StratumEntry::MESH_SIZE.as_str(),
                expected: MeshLayout2651::SIZE,
                found,
            });
        }
        anchor(EntryId::CAMERA_FORWARD, CameraLayout::FORWARD)?;
        let instance = get(StratumEntry::BLOCK_TYPE_REGISTRY_INSTANCE)?;
        Ok(Self {
            tessellator_begin: address(StratumEntry::TESSELLATOR_BEGIN)?,
            tessellator_end: address(StratumEntry::TESSELLATOR_END)?,
            tessellator_clear: address(StratumEntry::TESSELLATOR_CLEAR)?,
            mesh_dtor: address(StratumEntry::MESH_DTOR)?,
            mesh_is_valid: address(StratumEntry::MESH_IS_VALID)?,
            mesh_render: address(StratumEntry::MESH_RENDER_MESH)?,
            mesh_data_clear: address(StratumEntry::MESH_DATA_CLEAR)?,
            mesh_data_dtor: address(StratumEntry::MESH_DATA_DTOR)?,
            mesh_data_move_assign: address(StratumEntry::MESH_DATA_MOVE_ASSIGN)?,
            block_tessellator_ctor: address(StratumEntry::BLOCK_TESSELLATOR_CTOR)?,
            block_tessellator_dtor: address(StratumEntry::BLOCK_TESSELLATOR_DTOR)?,
            block_tessellator_set_region: address(StratumEntry::BLOCK_TESSELLATOR_SET_REGION)?,
            block_tessellator_tessellate_in_world: address(StratumEntry::BLOCK_TESSELLATOR_TESSELLATE_IN_WORLD)?,
            block_cache_get_block: address(StratumEntry::BLOCK_TESSELLATOR_CACHE_GET_BLOCK)?,
            block_source_get_block: address(StratumEntry::BLOCK_SOURCE_GET_BLOCK)?,
            block_graphics_get_for_block: address(StratumEntry::BLOCK_GRAPHICS_GET_FOR_BLOCK)?,
            transform_block: address(StratumEntry::TRANSFORM_BLOCK)?,
            has_transformable_wall_states: address(StratumEntry::HAS_TRANSFORMABLE_WALL_STATES)?,
            transform_wall_connection_types: address(StratumEntry::TRANSFORM_WALL_CONNECTION_TYPES)?,
            has_transformable_connection_states: address(StratumEntry::HAS_TRANSFORMABLE_CONNECTION_STATES)?,
            transform_connection_states: address(StratumEntry::TRANSFORM_CONNECTION_STATES)?,
            get_tessellation_policy: address(StratumEntry::BIOME_COLOR_SAMPLING_GET_TESSELLATION_POLICY)?,
            block_type_registry: (instance + StratumEntry::REGISTRY_IN_OWNER) as usize,
            lookup_by_name: address(StratumEntry::BLOCK_TYPE_REGISTRY_LOOKUP_BY_NAME)?,
            string_byte_input_vtable: address(StratumEntry::STRING_BYTE_INPUT_VTABLE)?,
            nbt_read: address(StratumEntry::NBT_IO_READ)?,
            try_get_block_from_nbt: address(StratumEntry::BLOCK_SERIALIZATION_UTILS_TRY_GET_BLOCK_FROM_NBT)?,
            compound_tag_dtor: address(StratumEntry::COMPOUND_TAG_DTOR)?,
            set_entity_constants: address(StratumEntry::ACTOR_SHADER_MANAGER_SET_ENTITY_CONSTANTS)?,
            matrix_stack_push: address(StratumEntry::MATRIX_STACK_PUSH)?,
            matrix_stack_ref_dtor: address(StratumEntry::MATRIX_STACK_REF_DTOR)?,
            entity_constant_buffer: get(StratumEntry::GLOBAL_CONSTANT_BUFFERS_ENTITY_CONSTANT_BUFFER)?,
            screen_tessellator: get(StratumEntry::SCREEN_CONTEXT_TESSELLATOR)?,
            atlas_texture: get(StratumEntry::LEVEL_RENDERER_ATLAS_TEXTURE)?,
            frame_views: get(StratumEntry::FRAME_RENDER_OBJECT_VIEWS)?,
            item_in_hand_renderer_slot: get(StratumEntry::CLIENT_INSTANCE_GET_ITEM_IN_HAND_RENDERER)?,
            blend_material: get(StratumEntry::ITEM_IN_HAND_RENDERER_MAT_BLEND_BLOCK)?,
            render_layer: get(StratumEntry::BLOCK_GRAPHICS_RENDER_LAYER)?,
            tessellation_policy_get_slot: get(StratumEntry::TESSELLATION_POLICY_GET)?,
            tint_method: get(StratumEntry::BLOCK_TYPE_TINT_METHOD)?,
            heap_new,
            heap_delete,
        })
    }
}
