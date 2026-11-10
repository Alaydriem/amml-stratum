use std::path::PathBuf;

use super::ImportFailure;
use crate::structs::library::NewStructure;

// The outcome of one import request.
#[derive(Debug, Clone, PartialEq)]
pub enum ImportResult {
    // The bytes are stored as `stored_file` in the structures folder.
    Ready { source: PathBuf, stored_file: String, structure: NewStructure },
    Failed { source: PathBuf, reason: ImportFailure },
}
