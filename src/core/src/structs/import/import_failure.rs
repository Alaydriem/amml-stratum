use crate::errors::StructureError;

// Why a file was not imported.
// The Display text is for the log only; the UI shows its own translated message.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ImportFailure {
    // The file could not be opened, read, or stored.
    #[error("the file could not be read or stored")]
    Unreadable,
    #[error("{0}")]
    NotStructure(StructureError),
    #[error("the file is {bytes} bytes, past the size limit")]
    TooBig { bytes: u64 },
}
