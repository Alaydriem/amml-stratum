use std::mem::offset_of;

use amml_game::{EntryId, GameEntries};

use crate::errors::GameLayoutError;
use crate::game::GameModeLayout;
use crate::types::StratumEntry;

/// The resolved game code and offsets of the easy place use path besides the packets and the inventory: the client's
/// local player, the game mode, the player's view, the level's hit, the blocks of a region, the item a block gives,
/// and the HUD's hotbar select. A test fills it with fake functions.
#[derive(Clone, Copy, Debug)]
pub struct UseCalls {
    /// Vtable slot of `LocalPlayer* ClientInstance::getLocalPlayer() const`.
    pub local_player_slot: u64,
    /// Offset of `Player& GameMode::mPlayer`, the anchor of the game mode.
    pub game_mode_player: u64,
    /// Vtable slot of `bool GameMode::buildBlock(BlockPos const&, uchar, HandSlot, bool)`.
    pub build_block_slot: u64,
    /// `float GameMode::getMaxPickRange()`.
    pub get_max_pick_range: usize,
    /// `Vec3 Actor::getViewVector(float) const`, returned through a hidden pointer.
    pub get_view_vector: usize,
    /// `Vec3 Actor::getAttachPos(ActorLocation, float) const`, returned through a hidden pointer.
    pub get_attach_pos: usize,
    /// Offset in `Actor` of the pointer to its `AABBShapeComponent`, whose `AABB` comes first.
    pub aabb_shape_component: u64,
    /// Vtable slot of `HitResult& Level::getHitResult()`.
    pub level_get_hit_result_slot: u64,
    /// `Block const& BlockSource::getBlock(BlockPos const&) const`.
    pub block_source_get_block: usize,
    /// Vtable slot of `bool BlockType::canBeBuiltOver(Block const&, BlockSource const&, BlockPos const&) const`.
    pub can_be_built_over_slot: u64,
    /// Vtable slot of `bool BlockType::isInteractiveBlock() const`.
    pub is_interactive_block_slot: u64,
    /// Vtable slot of `ItemInstance BlockType::asItemInstance(Block const&, BlockActor const*) const`, returned
    /// through a hidden pointer.
    pub as_item_instance_slot: u64,
    /// `ItemStackBase::~ItemStackBase()`, the body, which an `ItemInstance` uses as its whole destructor.
    pub item_stack_base_dtor: usize,
    /// `void HudScreenController::_handleSlotSelection(int, ContainerID)`.
    pub handle_slot_selection: usize,
}

impl UseCalls {
    /// The calls of the build `entries` was resolved for; an error names the first entry that is missing, or the
    /// `GameMode.mPlayer` anchor when it is not where the game mode mirror has `mPlayer`.
    pub fn new(entries: &GameEntries) -> Result<Self, GameLayoutError> {
        let get = |id: EntryId| entries.get(id).ok_or(GameLayoutError::Missing { entry: id.as_str() });
        let game_mode_player = get(StratumEntry::GAME_MODE_PLAYER)?;
        let expected = offset_of!(GameModeLayout, player) as u64;
        if game_mode_player != expected {
            return Err(GameLayoutError::Misfit {
                entry: StratumEntry::GAME_MODE_PLAYER.as_str(),
                expected,
                found: game_mode_player,
            });
        }
        Ok(Self {
            local_player_slot: get(EntryId::CLIENT_INSTANCE_GET_LOCAL_PLAYER)?,
            game_mode_player,
            build_block_slot: get(StratumEntry::GAME_MODE_BUILD_BLOCK)?,
            get_max_pick_range: get(StratumEntry::GAME_MODE_GET_MAX_PICK_RANGE)? as usize,
            get_view_vector: get(StratumEntry::ACTOR_GET_VIEW_VECTOR)? as usize,
            get_attach_pos: get(EntryId::ACTOR_GET_ATTACH_POS)? as usize,
            aabb_shape_component: get(StratumEntry::ACTOR_AABB_SHAPE_COMPONENT)?,
            level_get_hit_result_slot: get(StratumEntry::LEVEL_GET_HIT_RESULT)?,
            block_source_get_block: get(StratumEntry::BLOCK_SOURCE_GET_BLOCK)? as usize,
            can_be_built_over_slot: get(StratumEntry::BLOCK_TYPE_CAN_BE_BUILT_OVER)?,
            is_interactive_block_slot: get(StratumEntry::BLOCK_TYPE_IS_INTERACTIVE_BLOCK)?,
            as_item_instance_slot: get(StratumEntry::BLOCK_TYPE_AS_ITEM_INSTANCE)?,
            item_stack_base_dtor: get(StratumEntry::ITEM_STACK_BASE_DTOR)? as usize,
            handle_slot_selection: get(StratumEntry::HUD_SCREEN_CONTROLLER_HANDLE_SLOT_SELECTION)? as usize,
        })
    }
}
