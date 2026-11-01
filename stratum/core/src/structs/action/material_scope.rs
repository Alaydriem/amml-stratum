use crate::structs::library::{ProjectId, StructureId};

// What a material list covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialScope {
    Project(ProjectId),
    Structure(StructureId),
}
