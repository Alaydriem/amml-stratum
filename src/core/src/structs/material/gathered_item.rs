use super::ItemId;

// A ticked row of a material list: the item and the count the list needed when it was ticked.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GatheredItem {
    pub item: ItemId,
    pub count: u64,
}
