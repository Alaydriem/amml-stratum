use std::path::PathBuf;

// Where the preview reads from: the game's resource packs folder, None when it was not found, and the folder of
// the stored structure files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewSource {
    pub packs_dir: Option<PathBuf>,
    pub structures_dir: PathBuf,
}

impl PreviewSource {
    pub fn new(packs_dir: Option<PathBuf>, structures_dir: PathBuf) -> Self {
        Self { packs_dir, structures_dir }
    }
}
