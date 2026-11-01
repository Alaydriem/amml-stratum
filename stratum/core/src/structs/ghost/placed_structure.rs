use std::sync::Arc;

use crate::structs::structure::Structure;

// One structure of a build at the world position of its cell (0, 0, 0), before the build turns.
#[derive(Debug, Clone)]
pub struct PlacedStructure {
    pub structure: Arc<Structure>,
    pub origin: [i32; 3],
}

impl PlacedStructure {
    // The same loaded structure at the same position.
    pub fn same_as(&self, other: &PlacedStructure) -> bool {
        self.origin == other.origin && Arc::ptr_eq(&self.structure, &other.structure)
    }
}
