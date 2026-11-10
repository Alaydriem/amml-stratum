use crate::errors::{LibraryError, MarkerError, StructureError};
use crate::structs::import::ImportFailure;

// Something the user should be told. The snapshot keeps the newest few for a short time.
#[derive(Debug, Clone, PartialEq)]
pub enum ImportNotice {
    ImportFailed { file: String, reason: ImportFailure },
    // The library refused a file that imported well; the stored copy was removed.
    ImportRefused { file: String, error: LibraryError },
    ImportQueueFull,
    // The inbox file was imported but could not be removed.
    InboxFileKept { file: String },
    // The structure reader has too many loads waiting; the request can be made again.
    MaterialsBusy,
    NotInWorld,
    MarkerRefused(MarkerError),
    LibraryRefused(LibraryError),
    MaterialsFailed(StructureError),
    // The library file could not be read, so changes are not saved this session.
    LibraryUnavailable,
    // The marker file of the loaded world could not be read, so its markers are not saved.
    MarkersUnavailable,
    // Some wanted ghost sections got no mesh slot, or took the slot of a farther one.
    GhostCut,
    // The active marker's build has more distinct block states than the ghost palette holds, so no ghost is shown.
    GhostRefused,
}
