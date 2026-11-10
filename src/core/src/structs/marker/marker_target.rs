use crate::structs::library::{ProjectId, StructureId};

// What a marker shows: a whole project or one structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MarkerTarget {
    Project(ProjectId),
    Structure(StructureId),
}
