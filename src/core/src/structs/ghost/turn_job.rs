use std::sync::Arc;

use crate::structs::marker::Rotation;
use crate::structs::structure::Structure;

// A request to turn the structures of a build, in build piece order.
#[derive(Debug, Clone)]
pub struct TurnJob {
    pub ticket: u64,
    pub rotation: Rotation,
    pub sources: Vec<Arc<Structure>>,
}
