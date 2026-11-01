use std::path::{Path, PathBuf};

use crate::structs::preview::PreviewSource;

// Finds the folders the preview reads from in the installed game.
pub struct SourceLocator;

impl SourceLocator {
    // Looks for `data/resource_packs` holding `vanilla/blocks.json`, first under the game's working folder, then
    // under each folder above `data_dir` (the mod's data folder sits inside the game folder).
    pub fn locate(data_dir: &Path) -> PreviewSource {
        let mut roots: Vec<PathBuf> = std::env::current_dir().into_iter().collect();
        roots.extend(data_dir.ancestors().map(Path::to_path_buf));
        let packs_dir = roots
            .into_iter()
            .map(|root| root.join("data").join("resource_packs"))
            .find(|dir| dir.join("vanilla").join("blocks.json").is_file());
        PreviewSource::new(packs_dir, data_dir.join("structures"))
    }
}
