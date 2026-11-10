use stratum_core::structs::easy_place::InventorySlot;

use crate::game::{ItemName, PlayerInventory};

/// Reads the player's 36 inventory slots, 0 to 8 the hotbar and 9 to 35 the main inventory, as item names and counts.
/// Each name views the item's own registered name, so nothing is copied; the views stay valid for the decide call
/// that follows on the game thread. Read-only.
pub struct InventoryReader;

impl InventoryReader {
    /// The slots a use call hands the core.
    pub const CAPACITY: usize = 36;

    /// The slots, how many were filled (0 when the container is empty), and the selected slot (0 when it is out of
    /// range).
    pub fn read<'a>(inventory: &'a PlayerInventory) -> ([InventorySlot<'a>; Self::CAPACITY], usize, u8) {
        let mut slots = [InventorySlot { item: &[], count: 0 }; Self::CAPACITY];
        let mut count = 0;
        while count < Self::CAPACITY {
            let Some(stack) = inventory.item(count) else { break };
            // SAFETY: a stack of the live container; its item counter is live while the stack is
            #[allow(unsafe_code)]
            let item = unsafe { stack.base.item() };
            if !item.is_null() && stack.base.count != 0 {
                // SAFETY: the stack's live Item, whose name lives as long as the item registry
                #[allow(unsafe_code)]
                let name = unsafe { ItemName::bytes_of(item) }.unwrap_or(&[]);
                slots[count] = InventorySlot { item: name, count: u32::from(stack.base.count) };
            }
            count += 1;
        }
        let selected = inventory.selected().filter(|selected| (0..count as i32).contains(selected)).unwrap_or(0) as u8;
        (slots, count, selected)
    }
}
