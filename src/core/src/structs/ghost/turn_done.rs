use std::sync::Arc;

use crate::structs::marker::Rotation;
use crate::structs::structure::Structure;

// The structures of a `TurnJob`, turned. `turned[n]` is `sources[n]` turned by `rotation`.
#[derive(Debug, Clone)]
pub struct TurnDone {
    pub ticket: u64,
    pub rotation: Rotation,
    pub sources: Vec<Arc<Structure>>,
    pub turned: Vec<Arc<Structure>>,
}

impl TurnDone {
    // True when this turns exactly `sources`, the same loaded structures in the same order, by
    // `rotation`.
    pub fn turns(&self, sources: &[Arc<Structure>], rotation: Rotation) -> bool {
        self.rotation == rotation
            && self.sources.len() == sources.len()
            && self.sources.iter().zip(sources).all(|(a, b)| Arc::ptr_eq(a, b))
    }

    // Heap bytes of the turned layers.
    pub fn heap_bytes(&self) -> u64 {
        self.turned
            .iter()
            .map(|structure| structure.primary.heap_bytes().saturating_add(structure.secondary.heap_bytes()))
            .fold(0_u64, u64::saturating_add)
    }
}
