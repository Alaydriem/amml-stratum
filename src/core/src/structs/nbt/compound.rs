use super::NbtTag;

// Named entries in the order they appear in the file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NbtCompound {
    entries: Vec<(String, NbtTag)>,
}

impl NbtCompound {
    pub fn new(entries: Vec<(String, NbtTag)>) -> Self {
        Self { entries }
    }

    // First entry with this name.
    pub fn get(&self, name: &str) -> Option<&NbtTag> {
        self.entries.iter().find(|(entry, _)| entry == name).map(|(_, tag)| tag)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &NbtTag)> {
        self.entries.iter().map(|(name, tag)| (name.as_str(), tag))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
