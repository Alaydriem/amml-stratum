use stratum_core::structs::material::{ItemId, ItemYield, MaterialLine};
use stratum_core::structs::structure::{BlockState, BlockStates, StateValue};

// Builds block states, item ids and expected item yields for material tests.
pub struct BlockFactory;

impl BlockFactory {
    pub fn block(name: &str, states: &[(&str, StateValue)]) -> BlockState {
        let pairs = states.iter().map(|(key, value)| ((*key).to_string(), value.clone())).collect();
        BlockState::new(format!("minecraft:{name}"), BlockStates::new(pairs))
    }

    pub fn plain(name: &str) -> BlockState {
        Self::block(name, &[])
    }

    // A block in another namespace, written out in full.
    pub fn namespaced(full_name: &str) -> BlockState {
        BlockState::new(full_name.to_string(), BlockStates::default())
    }

    pub fn item(name: &str) -> ItemId {
        ItemId(format!("minecraft:{name}"))
    }

    pub fn one(name: &str, count: u32) -> ItemYield {
        ItemYield::One(Self::item(name), count)
    }

    pub fn two(first: &str, first_count: u32, second: &str, second_count: u32) -> ItemYield {
        ItemYield::Two(Self::item(first), first_count, Self::item(second), second_count)
    }

    // A material line with the default stack size.
    pub fn line(name: &str, count: u64) -> MaterialLine {
        MaterialLine { item: Self::item(name), count, stack_size: 64 }
    }
}
