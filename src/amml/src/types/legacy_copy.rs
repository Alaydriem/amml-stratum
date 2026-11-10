use super::FolderCopy;

// The result of the one-time copy of the old LeviLamina folders: the config folder, then the data folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyCopy {
    pub folders: Vec<FolderCopy>,
}

impl LegacyCopy {
    // one line per folder, each with whether it is a warning
    pub fn lines(&self) -> Vec<(bool, String)> {
        self.folders.iter().map(FolderCopy::line).collect()
    }
}
