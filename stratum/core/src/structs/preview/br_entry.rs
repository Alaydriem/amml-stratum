// One file inside a `.brarchive`: its name and where its bytes sit in the archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrEntry {
    pub name: String,
    // Offset from the start of the archive file.
    pub start: usize,
    pub len: usize,
}

impl BrEntry {
    pub fn new(name: String, start: usize, len: usize) -> Self {
        Self { name, start, len }
    }

    // The entry's bytes, or None when the archive is shorter than the entry says.
    pub fn bytes<'a>(&self, archive: &'a [u8]) -> Option<&'a [u8]> {
        archive.get(self.start..self.start.checked_add(self.len)?)
    }
}
