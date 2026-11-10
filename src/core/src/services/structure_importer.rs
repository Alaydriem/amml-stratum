use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use super::StructureParser;
use crate::config::ImportConfig;
use crate::structs::import::{ImportFailure, ImportRequest, ImportResult};
use crate::structs::library::{NameLimit, NewStructure};

// Reads, checks and stores one structure file. Runs on the import worker thread.
pub(crate) struct StructureImporter {
    store_dir: PathBuf,
    max_file_bytes: u64,
}

impl StructureImporter {
    const EXTENSION: &'static str = "mcstructure";
    const MAX_STEM_CHARS: usize = 100;
    const MAX_NAME_ATTEMPTS: u32 = 10_000;

    pub(crate) fn new(store_dir: PathBuf, config: &ImportConfig) -> Self {
        Self { store_dir, max_file_bytes: config.max_file_bytes }
    }

    pub(crate) fn import(&self, request: &ImportRequest) -> ImportResult {
        match self.try_import(&request.source) {
            Ok((stored_file, structure)) => {
                ImportResult::Ready { source: request.source.clone(), stored_file, structure }
            }
            Err(reason) => ImportResult::Failed { source: request.source.clone(), reason },
        }
    }

    fn try_import(&self, source: &Path) -> Result<(String, NewStructure), ImportFailure> {
        let bytes = self.read_limited(source)?;
        let parsed = StructureParser::parse(&bytes).map_err(ImportFailure::NotStructure)?;
        let stem = source.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let stored_file = self.store(&Self::safe_stem(&stem), &bytes)?;
        let structure = NewStructure {
            display_name: Self::display_name(&stem),
            file_name: stored_file.clone(),
            size: parsed.size,
            block_count: parsed.block_count(),
            world_origin: parsed.world_origin,
            project: None,
        };
        Ok((stored_file, structure))
    }

    fn read_limited(&self, source: &Path) -> Result<Vec<u8>, ImportFailure> {
        let file = std::fs::File::open(source).map_err(|e| Self::unreadable(source, &e))?;
        let length = file.metadata().map_err(|e| Self::unreadable(source, &e))?.len();
        if length > self.max_file_bytes {
            return Err(ImportFailure::TooBig { bytes: length });
        }
        let mut bytes = Vec::with_capacity(usize::try_from(length).unwrap_or(0));
        // One byte past the limit shows a file that grew after the size check.
        file.take(self.max_file_bytes.saturating_add(1))
            .read_to_end(&mut bytes)
            .map_err(|e| Self::unreadable(source, &e))?;
        let read = bytes.len() as u64;
        if read > self.max_file_bytes {
            return Err(ImportFailure::TooBig { bytes: read });
        }
        Ok(bytes)
    }

    // Writes `<store>/structures/<stem>.mcstructure`, or `<stem>-2`, `<stem>-3` ... when taken.
    fn store(&self, stem: &str, bytes: &[u8]) -> Result<String, ImportFailure> {
        let dir = self.store_dir.join("structures");
        std::fs::create_dir_all(&dir).map_err(|e| Self::unreadable(&dir, &e))?;
        for attempt in 1..=Self::MAX_NAME_ATTEMPTS {
            let name = match attempt {
                1 => format!("{stem}.{}", Self::EXTENSION),
                n => format!("{stem}-{n}.{}", Self::EXTENSION),
            };
            let path = dir.join(&name);
            match std::fs::OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(mut file) => {
                    let written = file.write_all(bytes).and_then(|()| file.sync_all());
                    if let Err(e) = written {
                        drop(file);
                        let _ = std::fs::remove_file(&path);
                        return Err(Self::unreadable(&path, &e));
                    }
                    return Ok(name);
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => return Err(Self::unreadable(&path, &e)),
            }
        }
        curia::error!("no free stored file name", { stem: stem.to_string() });
        Err(ImportFailure::Unreadable)
    }

    fn display_name(stem: &str) -> String {
        let name: String = stem.trim().chars().take(NameLimit::MAX_CHARS).collect();
        if name.trim().is_empty() { "Structure".to_string() } else { name }
    }

    // Keeps letters, digits, spaces and most punctuation; path and device characters become `_`.
    fn safe_stem(stem: &str) -> String {
        let cleaned: String = stem
            .chars()
            .take(Self::MAX_STEM_CHARS)
            .map(|c| if c.is_control() || "/\\:*?\"<>|".contains(c) { '_' } else { c })
            .collect();
        let trimmed = cleaned.trim_end_matches(['.', ' ']);
        if trimmed.is_empty() { "structure".to_string() } else { trimmed.to_string() }
    }

    fn unreadable(path: &Path, error: &std::io::Error) -> ImportFailure {
        curia::error!("import file error", { path: path.display().to_string(), error: error.to_string() });
        ImportFailure::Unreadable
    }
}
