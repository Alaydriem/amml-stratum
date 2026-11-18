use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use super::AtomicFile;
use crate::errors::StoreError;
use crate::structs::build::BuildSettingsBook;
use crate::structs::build::settings_entry::BuildSettingsEntry;
use crate::structs::build::settings_file::BuildSettingsFile;
use crate::structs::library::StructureId;

// The build settings of every structure, in one HCL file.
pub struct BuildSettingsStore {
    dir: PathBuf,
}

impl BuildSettingsStore {
    const FILE_NAME: &'static str = "build.hcl";

    pub fn new(dir: PathBuf) -> Self {
        Self { dir }
    }

    // The settings file.
    pub fn path(&self) -> PathBuf {
        self.dir.join(Self::FILE_NAME)
    }

    // A missing file gives an empty book. A file that is not UTF-8 or does not parse is kept as
    // `build.hcl.bad-<unix seconds>` and an empty book is returned. Rates are pulled into range, and entries
    // past `capacity` are dropped, the oldest first.
    pub fn load(&self, capacity: usize) -> Result<BuildSettingsBook, StoreError> {
        let path = self.path();
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(BuildSettingsBook::new(capacity)),
            Err(e) => return Err(StoreError::Io(e.to_string())),
        };
        match Self::parse(&bytes) {
            Ok(file) => Ok(Self::book_of(file, capacity)),
            Err(error) => {
                let seconds = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
                let kept = self.dir.join(format!("{}.bad-{seconds}", Self::FILE_NAME));
                std::fs::rename(&path, &kept).map_err(|e| StoreError::Io(e.to_string()))?;
                curia::error!("build settings file is corrupt and was kept", {
                    error: error.to_string(),
                    kept: kept.display().to_string(),
                });
                Ok(BuildSettingsBook::new(capacity))
            }
        }
    }

    pub fn save(&self, book: &BuildSettingsBook) -> Result<(), StoreError> {
        let (path, bytes) = self.encode(book)?;
        std::fs::create_dir_all(&self.dir).map_err(|e| StoreError::Io(e.to_string()))?;
        AtomicFile::write(&path, &bytes).map_err(|e| StoreError::Io(e.to_string()))
    }

    // The target path and the file bytes, for a writer on another thread.
    pub fn encode(&self, book: &BuildSettingsBook) -> Result<(PathBuf, Vec<u8>), StoreError> {
        let file = BuildSettingsFile {
            settings: book.entries().iter().map(|(id, settings)| BuildSettingsEntry::new(id.0, *settings)).collect(),
            owed: book.owed().to_vec(),
        };
        let text = hcl::to_string(&file).map_err(|e| StoreError::Parse(e.to_string()))?;
        Ok((self.path(), text.into_bytes()))
    }

    fn parse(bytes: &[u8]) -> Result<BuildSettingsFile, StoreError> {
        let text = std::str::from_utf8(bytes).map_err(|e| StoreError::Parse(e.to_string()))?;
        hcl::from_str::<BuildSettingsFile>(text).map_err(|e| StoreError::Parse(e.to_string()))
    }

    fn book_of(file: BuildSettingsFile, capacity: usize) -> BuildSettingsBook {
        let entries = file.settings.iter().map(|entry| (StructureId(entry.structure), entry.settings())).collect();
        BuildSettingsBook::from_entries(capacity, entries).with_owed(file.owed)
    }
}
