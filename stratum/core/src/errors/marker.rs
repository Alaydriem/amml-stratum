use crate::structs::library::NameLimit;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MarkerError {
    #[error("marker not found")]
    NotFound,
    #[error("name is empty")]
    NameEmpty,
    #[error("name is longer than {} characters", NameLimit::MAX_CHARS)]
    NameTooLong,
    #[error("world has {capacity} markers, the most allowed")]
    Full { capacity: usize },
}
