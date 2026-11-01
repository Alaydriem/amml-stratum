// One inventory slot. Slots 0 to 8 are the hotbar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InventorySlot<'a> {
    pub item: &'a [u8],
    pub count: u32,
}
