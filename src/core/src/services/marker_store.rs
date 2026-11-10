use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::AtomicFile;
use crate::errors::StoreError;
use crate::structs::marker::{MarkerSet, WorldKey};

// One HCL file per world and dimension, named by `WorldKey::file_stem`.
pub struct MarkerStore {
    dir: PathBuf,
}

impl MarkerStore {
    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    // A missing file gives an empty set. A file that is not UTF-8 or does not parse is kept as
    // `<stem>.hcl.bad-<unix seconds>` and an empty set is returned. The id counter is raised past
    // every id in the file.
    pub fn load(&self, world: &WorldKey) -> Result<MarkerSet, StoreError> {
        let name = Self::file_name(world);
        let path = self.dir.join(&name);
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(MarkerSet::new(world.clone())),
            Err(e) => return Err(StoreError::Io(e.to_string())),
        };
        let parsed = String::from_utf8(bytes)
            .map_err(|e| e.to_string())
            .and_then(|text| hcl::from_str::<MarkerSet>(&text).map_err(|e| e.to_string()));
        match parsed {
            Ok(mut set) => {
                set.world = world.clone();
                set.clamp_counters();
                set.clamp_views();
                Ok(set)
            }
            Err(error) => {
                let seconds = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
                let kept = self.dir.join(format!("{name}.bad-{seconds}"));
                std::fs::rename(&path, &kept).map_err(|e| StoreError::Io(e.to_string()))?;
                curia::error!("marker file is corrupt and was kept", {
                    error: error,
                    kept: kept.display().to_string(),
                });
                Ok(MarkerSet::new(world.clone()))
            }
        }
    }

    // The file a world is saved in.
    pub fn path_of(&self, world: &WorldKey) -> PathBuf {
        self.dir.join(Self::file_name(world))
    }

    // Reads a set from file bytes that are not on disk yet. The id counter is raised like in `load`.
    pub fn decode(&self, world: &WorldKey, bytes: &[u8]) -> Result<MarkerSet, StoreError> {
        let text = std::str::from_utf8(bytes).map_err(|e| StoreError::Parse(e.to_string()))?;
        let mut set = hcl::from_str::<MarkerSet>(text).map_err(|e| StoreError::Parse(e.to_string()))?;
        set.world = world.clone();
        set.clamp_counters();
        set.clamp_views();
        Ok(set)
    }

    pub fn save(&self, set: &MarkerSet) -> Result<(), StoreError> {
        let (path, bytes) = self.encode(set)?;
        std::fs::create_dir_all(&self.dir).map_err(|e| StoreError::Io(e.to_string()))?;
        AtomicFile::write(&path, &bytes).map_err(|e| StoreError::Io(e.to_string()))
    }

    // The target path and the file bytes, for a writer on another thread.
    pub fn encode(&self, set: &MarkerSet) -> Result<(PathBuf, Vec<u8>), StoreError> {
        let text = hcl::to_string(set).map_err(|e| StoreError::Parse(e.to_string()))?;
        Ok((self.dir.join(Self::file_name(&set.world)), text.into_bytes()))
    }

    fn file_name(world: &WorldKey) -> String {
        format!("{}.hcl", world.file_stem())
    }
}
