use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::AtomicFile;
use crate::errors::StoreError;
use crate::structs::library::Library;

pub struct LibraryStore {
    dir: PathBuf,
}

impl LibraryStore {
    const FILE_NAME: &'static str = "library.hcl";

    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    // The library file.
    pub fn path(&self) -> PathBuf {
        self.dir.join(Self::FILE_NAME)
    }

    // A missing file gives an empty library. A file that is not UTF-8 or does not parse is kept as
    // `library.hcl.bad-<unix seconds>` and an empty library is returned. Id counters are raised
    // past every id in the file.
    pub fn load(&self) -> Result<Library, StoreError> {
        let path = self.path();
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Library::new()),
            Err(e) => return Err(StoreError::Io(e.to_string())),
        };
        let parsed = String::from_utf8(bytes)
            .map_err(|e| e.to_string())
            .and_then(|text| hcl::from_str::<Library>(&text).map_err(|e| e.to_string()));
        match parsed {
            Ok(mut library) => {
                library.clamp_counters();
                Ok(library)
            }
            Err(error) => {
                let seconds = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
                let kept = self.dir.join(format!("{}.bad-{seconds}", Self::FILE_NAME));
                std::fs::rename(&path, &kept).map_err(|e| StoreError::Io(e.to_string()))?;
                curia::error!("library file is corrupt and was kept", {
                    error: error,
                    kept: kept.display().to_string(),
                });
                Ok(Library::new())
            }
        }
    }

    pub fn save(&self, library: &Library) -> Result<(), StoreError> {
        let (path, bytes) = self.encode(library)?;
        std::fs::create_dir_all(&self.dir).map_err(|e| StoreError::Io(e.to_string()))?;
        AtomicFile::write(&path, &bytes).map_err(|e| StoreError::Io(e.to_string()))
    }

    // The target path and the file bytes, for a writer on another thread.
    pub fn encode(&self, library: &Library) -> Result<(PathBuf, Vec<u8>), StoreError> {
        let text = hcl::to_string(library).map_err(|e| StoreError::Parse(e.to_string()))?;
        Ok((self.path(), text.into_bytes()))
    }
}
