#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BuildError {
    #[error("the command queue is full")]
    OutboxFull,
}
