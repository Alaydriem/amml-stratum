use super::ItemId;

// The items a player must gather for one block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemYield {
    None,
    One(ItemId, u32),
    Two(ItemId, u32, ItemId, u32),
}
