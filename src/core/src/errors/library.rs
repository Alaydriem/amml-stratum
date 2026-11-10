use crate::structs::library::NameLimit;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LibraryError {
    #[error("library item not found")]
    NotFound,
    #[error("name is empty")]
    NameEmpty,
    #[error("name is longer than {} characters", NameLimit::MAX_CHARS)]
    NameTooLong,
    #[error("library is full at {capacity} items")]
    Full { capacity: usize },
    #[error("file name is not a plain file name")]
    BadFileName,
}
