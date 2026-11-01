use amml_game::{EntryId, GameEntries};

use crate::errors::GameLayoutError;
use crate::types::StratumEntry;

/// The resolved game code and offsets of the player's inventory and its item stacks. A test fills it with fake
/// functions.
#[derive(Clone, Copy, Debug)]
pub struct PlayerCalls {
    /// Offset of `Player::mInventory` (a `std::unique_ptr<PlayerInventory>`).
    pub player_inventory: u64,
    /// Vtable slot of `void FillingContainer::swapSlots(int, int)`.
    pub swap_slots_slot: u64,
    /// Vtable slot of `void Container::setItem(int, ItemStack const&)`.
    pub set_item_slot: u64,
    /// The function the game calls to make an empty stack: `ItemStack::ItemStack()` on 26.51, and
    /// `ItemStackBase::ItemStackBase()` on 26.60, where the callers store the `ItemStack` vtable themselves.
    pub item_stack_empty_ctor: usize,
    /// `ItemStack::ItemStack(ItemStack const&)`.
    pub item_stack_copy_ctor: usize,
    /// The address of the `ItemStack` vtable; slot 0 is its scalar deleting destructor.
    pub item_stack_vtable: usize,
    /// Vtable slot of `void ItemStack::reinit(Item const&, int count, int auxValue)`.
    pub item_stack_reinit_slot: u64,
    /// `uchar ItemStackBase::getMaxStackSize() const`.
    pub get_max_stack_size: usize,
    /// Offset of `Actor::mLevel` (a `Level*`).
    pub actor_level: u64,
    /// Vtable slot of `ItemRegistryRef ILevel::getItemRegistry() const`, returned through a hidden pointer.
    pub level_get_item_registry_slot: u64,
    /// `Bedrock::NonOwnerPointer<CreativeItemRegistry> ItemRegistryRef::getCreativeItemRegistry()`, returned through a
    /// hidden pointer.
    pub get_creative_item_registry: usize,
    /// Offset of `CreativeItemRegistry::mCreativeItems`.
    pub creative_items: u64,
    /// `ActorRuntimeID Actor::getRuntimeID() const`, returned through a hidden pointer.
    pub get_runtime_id: usize,
    /// `GameType Player::getPlayerGameType() const`.
    pub get_player_game_type: usize,
}

impl PlayerCalls {
    /// The calls of the build `entries` was resolved for; an error names the first entry that is missing.
    pub fn new(entries: &GameEntries) -> Result<Self, GameLayoutError> {
        let get = |id: EntryId| entries.get(id).ok_or(GameLayoutError::Missing { entry: id.as_str() });
        Ok(Self {
            player_inventory: get(StratumEntry::PLAYER_PLAYER_INVENTORY)?,
            swap_slots_slot: get(StratumEntry::FILLING_CONTAINER_SWAP_SLOTS)?,
            set_item_slot: get(StratumEntry::FILLING_CONTAINER_SET_ITEM)?,
            item_stack_empty_ctor: get(StratumEntry::ITEM_STACK_EMPTY_CTOR)? as usize,
            item_stack_copy_ctor: get(StratumEntry::ITEM_STACK_COPY_CTOR)? as usize,
            item_stack_vtable: get(StratumEntry::ITEM_STACK_VTABLE)? as usize,
            item_stack_reinit_slot: get(StratumEntry::ITEM_STACK_REINIT)?,
            get_max_stack_size: get(StratumEntry::ITEM_STACK_BASE_GET_MAX_STACK_SIZE)? as usize,
            actor_level: get(EntryId::ACTOR_LEVEL)?,
            level_get_item_registry_slot: get(StratumEntry::LEVEL_GET_ITEM_REGISTRY)?,
            get_creative_item_registry: get(StratumEntry::ITEM_REGISTRY_REF_GET_CREATIVE_ITEM_REGISTRY)? as usize,
            creative_items: get(StratumEntry::CREATIVE_ITEM_REGISTRY_CREATIVE_ITEMS)?,
            get_runtime_id: get(StratumEntry::ACTOR_GET_RUNTIME_ID)? as usize,
            get_player_game_type: get(StratumEntry::PLAYER_GET_PLAYER_GAME_TYPE)? as usize,
        })
    }
}
