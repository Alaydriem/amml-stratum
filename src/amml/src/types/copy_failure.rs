use std::path::PathBuf;

// One file or folder of an old LeviLamina folder that was not copied, with the reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyFailure {
    pub path: PathBuf,
    pub error: String,
}
