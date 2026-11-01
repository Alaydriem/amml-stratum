#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StoreError {
    #[error("file access failed: {0}")]
    Io(String),
    #[error("file could not be parsed: {0}")]
    Parse(String),
}
