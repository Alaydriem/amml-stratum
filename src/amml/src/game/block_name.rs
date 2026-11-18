use std::ffi::c_void;

use crate::game::ItemName;

/// The full name of a `BlockType` (`minecraft:stone`), read from `BlockType::mNameInfo.mFullName`, a `HashedString`.
/// The name is given only when its hash matches the string, so a wrong offset gives no name instead of a wrong one.
pub struct BlockName;

impl BlockName {
    /// The name of `block_type`, whose `mFullName` is at `offset`; None for a null type, or a string that does not
    /// fit or does not match its hash.
    ///
    /// # Safety
    /// `block_type` must be null or a live `BlockType` of the game, with `mFullName` at `offset`.
    #[allow(unsafe_code)]
    pub unsafe fn of(block_type: *const c_void, offset: u64) -> Option<String> {
        // SAFETY: forwarded to the caller
        let bytes = unsafe { ItemName::bytes_at(block_type, offset) }?;
        std::str::from_utf8(bytes).ok().map(str::to_string)
    }
}
