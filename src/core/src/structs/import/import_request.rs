use std::path::PathBuf;

// One file for the import worker to read. The worker never changes or removes the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportRequest {
    pub source: PathBuf,
}
