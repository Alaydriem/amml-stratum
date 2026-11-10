use super::StructureError;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PreviewError {
    #[error("structure for the preview could not be read: {0}")]
    Structure(#[from] StructureError),
    #[error("structure is too large for the preview")]
    TooLarge,
    #[error("preview stopped before it finished")]
    Stopped,
}
