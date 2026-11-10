use std::ffi::c_void;

/// What the game gave for the NBT bytes of one palette entry. Each block is a `Block const*` of the world it was
/// looked up in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NbtLookup {
    /// The block with the entry's name and states.
    Exact(*const c_void),
    /// The block with the entry's name and the default states: the states were not a state set the game knows.
    Default(*const c_void),
    /// Neither: the bytes were empty, did not read, or named a block the game does not know.
    Unknown,
}

impl NbtLookup {
    /// The kind, as the log names it.
    pub fn kind(&self) -> &'static str {
        match self {
            NbtLookup::Exact(_) => "Exact",
            NbtLookup::Default(_) => "Default",
            NbtLookup::Unknown => "Unknown",
        }
    }

    /// The block; null for `Unknown`.
    pub fn block(&self) -> *const c_void {
        match self {
            NbtLookup::Exact(block) | NbtLookup::Default(block) => *block,
            NbtLookup::Unknown => std::ptr::null(),
        }
    }
}
