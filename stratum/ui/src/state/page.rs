use stratum_core::structs::library::{ProjectId, StructureId};

// The page the detail column shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    All,
    Unsorted,
    Project(ProjectId),
    Structure(StructureId),
}
