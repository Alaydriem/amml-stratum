#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum NbtError {
    #[error("input ends early at byte {offset}")]
    Truncated { offset: usize },
    #[error("unknown tag id {id} at byte {offset}")]
    UnknownTag { id: u8, offset: usize },
    #[error("nesting is deeper than the limit")]
    TooDeep,
    #[error("list or array claims {len} items, past the limit")]
    ListTooLong { len: u32 },
    #[error("decoded data is larger than the limit")]
    TooLarge,
    #[error("string at byte {offset} is not valid UTF-8")]
    BadUtf8 { offset: usize },
    #[error("root tag is not a compound")]
    RootNotCompound,
}
