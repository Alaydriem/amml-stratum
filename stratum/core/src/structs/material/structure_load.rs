use std::path::PathBuf;

use crate::structs::library::StructureId;

// A request to read and parse one stored structure file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructureLoad {
    pub id: StructureId,
    pub path: PathBuf,
}
