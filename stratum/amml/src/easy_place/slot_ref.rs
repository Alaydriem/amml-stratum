/// One slot of an item stack request: the container id of the request protocol, the slot, and the net id of the
/// stack the slot holds (0 for an empty slot).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotRef {
    pub container: u8,
    pub slot: u8,
    pub net_id: i32,
}

impl SlotRef {
    /// The container ids of the item stack request protocol.
    pub const HOTBAR: u8 = 28;
    pub const INVENTORY: u8 = 29;
    pub const CREATED_OUTPUT: u8 = 60;
    /// The slot of the created output container that a creative craft fills.
    pub const CREATED_OUTPUT_SLOT: u8 = 50;
}
