use stratum_core::structs::library::{ProjectId, StructureId};
use stratum_core::structs::marker::MarkerId;

// The library item a rename or delete dialog is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogItem {
    Project(ProjectId),
    Structure(StructureId),
    Marker(MarkerId),
}
