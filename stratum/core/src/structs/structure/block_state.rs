use super::BlockStates;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BlockState {
    pub name: String,
    pub states: BlockStates,
}

impl BlockState {
    pub fn new(name: String, states: BlockStates) -> Self {
        Self { name, states }
    }

    // True for blocks that hold no material: air and structure void.
    pub fn is_empty_block(&self) -> bool {
        self.name == "minecraft:air" || self.name == "minecraft:structure_void"
    }
}
