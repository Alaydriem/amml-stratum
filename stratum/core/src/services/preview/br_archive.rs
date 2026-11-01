use crate::structs::preview::BrEntry;

// Reads the index of a game `.brarchive` file. The layout: an 8 byte magic, a u32 entry count and a u32 version,
// then one 256 byte record per entry (a u8 name length, the name padded to 247 bytes, a u32 offset and a u32
// length), then the file bytes. Offsets count from the end of the records. All numbers are little endian.
pub struct BrArchive;

impl BrArchive {
    pub const MAGIC: [u8; 8] = [0x7d, 0x27, 0x25, 0xb1, 0xa0, 0x52, 0x70, 0x26];
    const HEADER: usize = 16;
    const RECORD: usize = 256;
    const NAME: usize = 247;
    // More entries than this is taken as a broken file.
    pub const MAX_ENTRIES: usize = 65_536;

    // The entries of an archive, or None when the bytes are not an archive.
    pub fn entries(bytes: &[u8]) -> Option<Vec<BrEntry>> {
        if bytes.get(..8)? != Self::MAGIC {
            return None;
        }
        let count = Self::u32_at(bytes, 8)? as usize;
        if count > Self::MAX_ENTRIES {
            return None;
        }
        let data = Self::HEADER.checked_add(count.checked_mul(Self::RECORD)?)?;
        let mut entries = Vec::with_capacity(count);
        for index in 0..count {
            let record = Self::HEADER + index * Self::RECORD;
            let name_len = usize::from(*bytes.get(record)?).min(Self::NAME);
            let name = std::str::from_utf8(bytes.get(record + 1..record + 1 + name_len)?).ok()?;
            let offset = Self::u32_at(bytes, record + 1 + Self::NAME)? as usize;
            let len = Self::u32_at(bytes, record + 1 + Self::NAME + 4)? as usize;
            entries.push(BrEntry::new(name.to_owned(), data.checked_add(offset)?, len));
        }
        Some(entries)
    }

    fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
        let raw: [u8; 4] = bytes.get(at..at + 4)?.try_into().ok()?;
        Some(u32::from_le_bytes(raw))
    }
}
