// The import queue is full or the worker is gone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("the import queue is full")]
pub struct ImportBusy;
