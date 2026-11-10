use std::ffi::c_void;

use amml_game::{ItemStackLayout, ItemStackNetIdVariantLayout, RawObject};

use crate::game::OwnedItemStack;
use crate::types::PlayerCalls;

// `void FillingContainer::swapSlots(int, int)`: this, the two slots
type SwapSlotsFn = unsafe extern "C" fn(*mut c_void, i32, i32);

// `void Container::setItem(int, ItemStack const&)`: this, the slot, the stack
type SetItemFn = unsafe extern "C" fn(*mut c_void, i32, *const ItemStackLayout);

/// The local player's `PlayerInventory` and the `FillingContainer` it holds: the selected slot, the stacks as they
/// are, read through the `ItemStack` mirror over the container's item vector, and the game's own slot swap.
pub struct PlayerInventory {
    supplies: *mut c_void,
    items: *mut c_void,
    swap_slots_slot: u64,
    set_item_slot: u64,
}

impl PlayerInventory {
    // PlayerInventory: two 8-byte listener bases, then mSelected (int); mInventory is a unique_ptr<Inventory>. The
    // kept bytes of the Player.playerInventory anchor hold both offsets
    const SELECTED: u64 = 0x10;
    const INVENTORY: u64 = 0xB8;
    // FillingContainer::mItems, the std::vector<ItemStack> that FillingContainer::swapSlots indexes (its first
    // pointer at +198h, 98h bytes a stack)
    const ITEMS: u64 = 0x198;

    /// The inventory of `player`; None for a null player, inventory or container.
    ///
    /// # Safety
    /// `player` must be null or the live local `Player` of the build `calls` was resolved for.
    #[allow(unsafe_code)]
    pub unsafe fn of_player(player: *mut c_void, calls: &PlayerCalls) -> Option<Self> {
        if player.is_null() {
            return None;
        }
        // SAFETY: the caller guarantees a live player; the anchor is the offset of its unique_ptr<PlayerInventory>
        let supplies = unsafe { RawObject::pointer(player, calls.player_inventory) }?;
        // SAFETY: a live PlayerInventory, whose unique_ptr<Inventory> sits at INVENTORY
        let items = unsafe { RawObject::pointer(supplies, Self::INVENTORY) }?;
        Some(Self { supplies, items, swap_slots_slot: calls.swap_slots_slot, set_item_slot: calls.set_item_slot })
    }

    /// The selected hotbar slot.
    pub fn selected(&self) -> Option<i32> {
        // SAFETY: the live PlayerInventory holds mSelected at SELECTED
        #[allow(unsafe_code)]
        unsafe {
            RawObject::read::<i32>(self.supplies, Self::SELECTED)
        }
    }

    /// The stack in `slot`; None for a slot past the container's end.
    pub fn item(&self, slot: usize) -> Option<&ItemStackLayout> {
        // SAFETY: the live container holds mItems (first, last, end) at ITEMS
        #[allow(unsafe_code)]
        let [first, last] = unsafe { RawObject::read::<[usize; 2]>(self.items, Self::ITEMS) }?;
        let count = last.checked_sub(first)? / size_of::<ItemStackLayout>();
        if first == 0 || slot >= count {
            return None;
        }
        // SAFETY: a slot inside the container's live item vector
        #[allow(unsafe_code)]
        unsafe {
            (first as *const ItemStackLayout).add(slot).as_ref()
        }
    }

    /// Puts a copy of `stack` into `slot` through the game's own `setItem`, which also tells the container's
    /// listeners.
    pub fn set_item(&self, slot: i32, stack: &OwnedItemStack) {
        // SAFETY: the live container, whose vtable holds setItem at the entry's slot
        #[allow(unsafe_code)]
        let Some(function) = (unsafe { RawObject::slot(self.items, self.set_item_slot) }) else { return };
        // SAFETY: the slot holds `void Container::setItem(int, ItemStack const&)`
        #[allow(unsafe_code)]
        let set: SetItemFn = unsafe { std::mem::transmute(function) };
        // SAFETY: the live container and a live stack the game copies
        #[allow(unsafe_code)]
        unsafe {
            set(self.items, slot, stack.as_ptr())
        };
    }

    /// Gives the stack in `slot` the net id the server gave it, as the C++ did after an accepted pick.
    pub fn set_net_id(&self, slot: usize, net_id: i32) {
        let Some(stack) = self.item(slot) else { return };
        let stack = std::ptr::from_ref(stack).cast_mut();
        // SAFETY: a stack of the live container's item vector, written on the game thread
        #[allow(unsafe_code)]
        unsafe {
            (*stack).net_id_variant = ItemStackNetIdVariantLayout::from_net_id(net_id)
        };
    }

    /// Swaps the stacks of slots `a` and `b` through the game's own swap, which also tells the container's
    /// listeners.
    pub fn swap_slots(&self, a: i32, b: i32) {
        // SAFETY: the live container, whose vtable holds swapSlots at the entry's slot
        #[allow(unsafe_code)]
        let Some(function) = (unsafe { RawObject::slot(self.items, self.swap_slots_slot) }) else { return };
        // SAFETY: the slot holds `void FillingContainer::swapSlots(int, int)`
        #[allow(unsafe_code)]
        let swap: SwapSlotsFn = unsafe { std::mem::transmute(function) };
        // SAFETY: the live container and two slots the caller read from it
        #[allow(unsafe_code)]
        unsafe {
            swap(self.items, a, b)
        };
    }
}
