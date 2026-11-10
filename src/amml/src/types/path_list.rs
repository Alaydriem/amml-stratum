use std::path::PathBuf;

use stratum_core::config::ImportConfig;

// The files of one picker answer. Bound: at most `MAX` paths, the deepest import queue; the paths past it are left
// out, because the worker would refuse them anyway.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathList {
    paths: Vec<PathBuf>,
}

impl PathList {
    pub const MAX: usize = ImportConfig::MAX_QUEUE;

    pub fn new(picked: Vec<String>) -> Self {
        if picked.len() > Self::MAX {
            curia::warn!("picked files past the import queue were left out", { picked: picked.len(), kept: Self::MAX });
        }
        Self { paths: picked.into_iter().take(Self::MAX).map(PathBuf::from).collect() }
    }

    pub fn into_paths(self) -> Vec<PathBuf> {
        self.paths
    }
}
