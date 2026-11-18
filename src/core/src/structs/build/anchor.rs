use crate::structs::library::StructureId;
use crate::structs::marker::{MarkerId, Rotation};

// The marker and the world a build was opened on. The build ends when any part of it changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildAnchor {
    pub world: String,
    pub dimension: i32,
    pub marker: MarkerId,
    pub origin: [i32; 3],
    pub rotation: Rotation,
    pub structure: StructureId,
}
