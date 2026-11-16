use amml_game::{EntryId, GameEntries};

use crate::types::ghost_calls::HeapDeleteFn;
use crate::types::{ChestPairCalls, SkullCalls, StratumEntry};

/// The resolved game code and offsets of the block-entity ghosts: the draw hook and its player filter, the dispatcher
/// and the render context fields, the actor make, load and render component, and the NBT read and tag release the load
/// shares with the palette lookups; with them the optional chest pair fields and skull draw. A test fills it with fake
/// functions.
#[derive(Clone, Copy, Debug)]
pub struct BlockActorCalls {
    /// `void LevelRendererCamera::renderBlockEntities(BaseActorRenderContext&, bool)`, the body every camera shares.
    pub render_block_entities_body: usize,
    /// The address of the `LevelRendererPlayer` vtable, the one camera the draw hook works for.
    pub level_renderer_player_vtable: usize,
    /// `BlockActorRenderDispatcher::render`, the form that takes a render position.
    pub dispatcher_render: usize,
    /// Offset of `mBlockEntityRenderDispatcher` in `BaseActorRenderContext`.
    pub context_dispatcher: u64,
    /// Offset of `mImpl` in `BaseActorRenderContext`.
    pub context_impl: u64,
    /// Offset of `mCameraTargetPosition` (a Vec3) in `BaseActorRenderContext::Impl`.
    pub impl_camera_target: u64,
    /// `shared_ptr<BlockActor> VanillaBlockActorFactory::createBlockActor(BlockPos const&, BlockType const&)`.
    pub create_block_actor: usize,
    /// `shared_ptr<BlockActor> BlockActor::loadStatic(BlockType const&, BlockPos const&, ILevel&,
    /// CompoundTag const&, DataLoadHelper&)`.
    pub load_static: usize,
    /// Vtable slot of `IVanillaRenderBlockActorComponent* BlockActor::_getRenderComponent()`.
    pub render_component_slot: u64,
    /// The address of the `DefaultDataLoadHelper` vtable.
    pub data_load_helper_vtable: usize,
    /// Vtable slot of `ClientLevel* ClientInstance::getLevel()`, the level a load takes.
    pub get_level_slot: u64,
    /// The address of the `StringByteInput` vtable, for the input the NBT read takes.
    pub string_byte_input_vtable: usize,
    /// `std::unique_ptr<CompoundTag> NbtIo::read(IDataInput&)`.
    pub nbt_read: usize,
    /// `CompoundTag::~CompoundTag()`, the destructor that does not free the object.
    pub compound_tag_dtor: usize,
    /// The game's sized operator delete, which frees the read tag.
    pub heap_delete: HeapDeleteFn,
    /// The large chest calls; `None` while `stratum.large_chests` is off.
    pub chest: Option<ChestPairCalls>,
    /// The skull calls; `None` while `stratum.skulls` is off.
    pub skull: Option<SkullCalls>,
}

impl BlockActorCalls {
    /// The calls of the build `entries` was resolved for; `None` when any entry of the feature is missing, so the
    /// feature is off while the block ghosts still draw. A missing chest or skull entry turns only that group off.
    /// `heap_delete` is the game heap the read tags are freed to.
    pub fn new(entries: &GameEntries, heap_delete: HeapDeleteFn) -> Option<Self> {
        let get = |id: EntryId| entries.get(id);
        let address = |id: EntryId| get(id).map(|value| value as usize);
        Some(Self {
            render_block_entities_body: address(StratumEntry::LEVEL_RENDERER_CAMERA_RENDER_BLOCK_ENTITIES_BODY)?,
            level_renderer_player_vtable: address(StratumEntry::LEVEL_RENDERER_PLAYER_VTABLE)?,
            dispatcher_render: address(StratumEntry::BLOCK_ACTOR_RENDER_DISPATCHER_RENDER)?,
            context_dispatcher: get(StratumEntry::BASE_ACTOR_RENDER_CONTEXT_BLOCK_ENTITY_RENDER_DISPATCHER)?,
            context_impl: get(StratumEntry::BASE_ACTOR_RENDER_CONTEXT_IMPL)?,
            impl_camera_target: get(StratumEntry::BASE_ACTOR_RENDER_CONTEXT_IMPL_CAMERA_TARGET_POSITION)?,
            create_block_actor: address(StratumEntry::VANILLA_BLOCK_ACTOR_FACTORY_CREATE_BLOCK_ACTOR)?,
            load_static: address(StratumEntry::BLOCK_ACTOR_LOAD_STATIC)?,
            render_component_slot: get(StratumEntry::BLOCK_ACTOR_GET_RENDER_COMPONENT)?,
            data_load_helper_vtable: address(StratumEntry::DEFAULT_DATA_LOAD_HELPER_VTABLE)?,
            get_level_slot: get(StratumEntry::CLIENT_INSTANCE_GET_LEVEL)?,
            string_byte_input_vtable: address(StratumEntry::STRING_BYTE_INPUT_VTABLE)?,
            nbt_read: address(StratumEntry::NBT_IO_READ)?,
            compound_tag_dtor: address(StratumEntry::COMPOUND_TAG_DTOR)?,
            heap_delete,
            chest: ChestPairCalls::new(entries),
            skull: SkullCalls::new(entries),
        })
    }
}
