#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NbtWriteError {
    #[error("a span tag points into bytes the writer does not have")]
    SpanNotWritable,
    #[error("a string of {len} bytes or a list of {len} items does not fit its length field")]
    TooLong { len: usize },
}
