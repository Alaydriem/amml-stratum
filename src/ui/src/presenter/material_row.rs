use stratum_core::structs::material::{ItemId, StackCount};

// One line of the materials table: the item, its readable name, how many are needed, the count in stacks and
// whether it is ticked as gathered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterialRow {
    pub item: ItemId,
    pub name: String,
    pub count: u64,
    pub stacks: StackCount,
    pub stack_size: u32,
    pub gathered: bool,
}
