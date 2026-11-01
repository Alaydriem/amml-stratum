use super::StructureRow;
use crate::structs::library::{ProjectId, ProjectKind};

// One project and its structures, sorted by name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRow {
    pub id: ProjectId,
    pub name: String,
    pub kind: ProjectKind,
    pub structures: Vec<StructureRow>,
}
