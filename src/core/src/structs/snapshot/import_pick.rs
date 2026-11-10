use crate::structs::library::{ProjectId, StructureId};

// The structures one file picker answer imported. A newer pick replaces it; inbox imports never touch it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportPick {
    // Grows with every pick, so the UI can tell a new pick from the one it has seen.
    pub serial: u64,
    // The project the structures landed in, or `None` for Unsorted.
    pub destination: Option<ProjectId>,
    // The structures the library accepted so far, in the order they arrived.
    pub structures: Vec<StructureId>,
}

impl ImportPick {
    pub fn new(serial: u64, destination: Option<ProjectId>) -> Self {
        Self { serial, destination, structures: Vec::new() }
    }
}
