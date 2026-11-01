use super::NbtError;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StructureError {
    #[error("structure NBT is invalid: {0}")]
    Nbt(#[from] NbtError),
    #[error("structure field {0} is missing or has the wrong type")]
    MissingField(&'static str),
    #[error("structure size is not three non-negative integers")]
    BadSize,
    #[error("block index list holds {found} entries, the size needs {expected}")]
    IndexCountMismatch { expected: u64, found: u64 },
    #[error("block index {index} is not in the palette")]
    PaletteIndexOutOfRange { index: i32 },
    #[error("structure palette holds {entries} block types, the limit is 65534")]
    PaletteTooLarge { entries: usize },
    #[error("structure volume {volume} is past the limit")]
    TooLarge { volume: u64 },
    #[error("structure file could not be read: {0}")]
    Unreadable(String),
    #[error("structure read stopped before it finished")]
    Stopped,
}
