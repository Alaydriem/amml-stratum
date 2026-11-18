use std::sync::Arc;

use crate::structs::library::StructureId;
use crate::structs::marker::Rotation;
use crate::structs::structure::Structure;

// A structure for the worker to place in the world: turned by the rotation around the cell at the origin.
#[derive(Debug, Clone)]
pub struct AssembleRequest {
    // Names the request, so an answer to an older one is dropped.
    pub serial: u64,
    pub id: StructureId,
    pub structure: Arc<Structure>,
    pub origin: [i32; 3],
    pub rotation: Rotation,
}
