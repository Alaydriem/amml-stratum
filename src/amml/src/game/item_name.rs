use std::ffi::c_void;

use amml_game::RawObject;

/// The full name of an `Item` (`minecraft:stone`), read from `Item::mFullName`, a `HashedString`. The name is given
/// only when `HashedString::computeHash` of the string is the hash the `HashedString` holds, so a wrong offset gives no name instead
/// of a wrong one.
pub struct ItemName;

impl ItemName {
    // Item::mFullName: after the vptr and the members from mItemParseVersion to mNamespace in the 26.51 header
    const FULL_NAME: u64 = 0x120;
    // HashedString: the hash, then the std::string
    const TEXT: u64 = 0x8;
    // an MSVC std::string: 16 bytes (inline text or the heap pointer), the size, the capacity
    const INLINE: usize = 15;
    const LIMIT: usize = 128;
    const HASH_START: u64 = 0xcbf2_9ce4_8422_2325;
    const HASH_PRIME: u64 = 0x0000_0100_0000_01b3;

    /// The name of `item`; None for a null item, or a string that does not fit or does not match its hash.
    ///
    /// # Safety
    /// `item` must be null or a live `Item` of the game.
    #[allow(unsafe_code)]
    pub unsafe fn of(item: *const c_void) -> Option<String> {
        // SAFETY: forwarded to the caller
        let bytes = unsafe { Self::bytes_of(item) }?;
        std::str::from_utf8(bytes).ok().map(str::to_string)
    }

    /// The name of `item` as a view of the game's own string, with no copy; None as for [`ItemName::of`].
    ///
    /// # Safety
    /// `item` must be null or a live `Item` of the game, and the view must not be used after the game frees the item
    /// (items live as long as the world's item registry).
    #[allow(unsafe_code)]
    pub unsafe fn bytes_of<'a>(item: *const c_void) -> Option<&'a [u8]> {
        if item.is_null() {
            return None;
        }
        // SAFETY: the caller guarantees a live Item; the HashedString lies inside it
        let hash = unsafe { RawObject::read::<u64>(item, Self::FULL_NAME) }?;
        // SAFETY: as above
        let [storage, _, size, capacity] =
            unsafe { RawObject::read::<[usize; 4]>(item, Self::FULL_NAME + Self::TEXT) }?;
        if size == 0 || size > capacity || size > Self::LIMIT {
            return None;
        }
        let text = if capacity <= Self::INLINE {
            item.cast::<u8>().wrapping_add((Self::FULL_NAME + Self::TEXT) as usize)
        } else {
            if storage == 0 {
                return None;
            }
            storage as *const u8
        };
        // SAFETY: the string's own text of `size` bytes, inline in the Item or in its heap block
        let bytes = unsafe { std::slice::from_raw_parts(text, size) };
        (Self::hash(bytes) == hash).then_some(bytes)
    }

    /// `HashedString::computeHash` as the 26.51 header writes it: for each `char`, `hash = c ^ (prime * hash)` (the
    /// multiply comes first, and the signed `char` is sign-extended), and 0 for an empty string. It is not FNV-1a.
    pub fn hash(bytes: &[u8]) -> u64 {
        if bytes.is_empty() {
            return 0;
        }
        bytes
            .iter()
            .fold(Self::HASH_START, |hash, byte| (*byte as i8 as i64 as u64) ^ Self::HASH_PRIME.wrapping_mul(hash))
    }
}
