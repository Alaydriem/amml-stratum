use super::{CopyFailure, SkipReason};

// What happened to one old LeviLamina folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CopyOutcome {
    // every file was copied
    Copied { files: usize },
    Skipped(SkipReason),
    // a file or folder could not be copied, so nothing was kept and the new folder is as it was
    Failed { failures: Vec<CopyFailure> },
}
