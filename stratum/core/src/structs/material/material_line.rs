use super::ItemId;

// One row of a material list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterialLine {
    pub item: ItemId,
    pub count: u64,
    pub stack_size: u32,
}
