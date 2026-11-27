use amml_api::{Inventory, ItemSnapshot};
use stratum_core::structs::easy_place::InventorySlot;

/// Reads the player's 36 inventory slots, 0 to 8 the hotbar and 9 to 35 the main inventory, as item names and counts,
/// in one loader call into a buffer the caller keeps, so the names live for the decide call that follows. Read-only.
pub struct InventoryReader;

impl InventoryReader {
    /// The slots a use call hands the core.
    pub const CAPACITY: usize = 36;

    /// Fills `buffer` with the slots (all empty when the loader refuses the read) and gives the selected slot, 0 when
    /// it cannot be read.
    pub fn read(inventory: Option<&Inventory<'_>>, buffer: &mut [Option<ItemSnapshot>; Self::CAPACITY]) -> u8 {
        let Some(inventory) = inventory else {
            buffer.iter_mut().for_each(|slot| *slot = None);
            return 0;
        };
        match inventory.slots() {
            Some(slots) => *buffer = slots,
            None => buffer.iter_mut().for_each(|slot| *slot = None),
        }
        inventory.selected_slot().filter(|slot| usize::from(*slot) < Self::CAPACITY).unwrap_or(0)
    }

    /// The core's view of `buffer`: each slot's name and count; an empty slot or one with no readable name is empty.
    pub fn slots(buffer: &[Option<ItemSnapshot>; Self::CAPACITY]) -> [InventorySlot<'_>; Self::CAPACITY] {
        std::array::from_fn(|index| match &buffer[index] {
            Some(snapshot) if snapshot.count != 0 => InventorySlot {
                item: snapshot.type_name.as_deref().map_or(&[][..], str::as_bytes),
                count: u32::from(snapshot.count),
            },
            _ => InventorySlot { item: &[], count: 0 },
        })
    }
}
