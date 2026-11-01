use stratum_core::structs::material::StackCount;

// One line of the materials table: the readable item name, how many are needed and the count in stacks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterialRow {
    pub name: String,
    pub count: u64,
    pub stacks: StackCount,
    pub stack_size: u32,
}
