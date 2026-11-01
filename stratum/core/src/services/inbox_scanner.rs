use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::structs::import::ImportFailure;

// Lists `.mcstructure` files in the inbox folder once they stop growing.
// Bound: at most 256 files are tracked and listed per scan; the tracking map is rebuilt from
// the files present on each scan, so a file that leaves the folder is forgotten.
pub struct InboxScanner {
    inbox: PathBuf,
    // Size at the previous scan, and whether the file was already listed.
    seen: Mutex<HashMap<PathBuf, (u64, bool)>>,
}

impl InboxScanner {
    pub const MAX_PER_SCAN: usize = 256;
    const MAX_FAILED_NAMES: u32 = 1000;

    pub fn new(inbox: PathBuf) -> Self {
        Self { inbox, seen: Mutex::new(HashMap::new()) }
    }

    // Appends the files whose size equals their size at the previous scan and that were not
    // listed before. A missing inbox lists nothing.
    pub fn scan(&self, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(&self.inbox) else {
            return;
        };
        let Ok(mut seen) = self.seen.lock() else {
            return;
        };
        let mut now: HashMap<PathBuf, (u64, bool)> = HashMap::new();
        for entry in entries.flatten() {
            if now.len() >= Self::MAX_PER_SCAN {
                break;
            }
            let path = entry.path();
            if !Self::is_structure_name(&path) {
                continue;
            }
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            if !metadata.is_file() {
                continue;
            }
            let size = metadata.len();
            let listed = match seen.get(&path) {
                Some((previous, listed)) if *previous == size => {
                    if !listed {
                        out.push(path.clone());
                    }
                    true
                }
                _ => false,
            };
            now.insert(path, (size, listed));
        }
        *seen = now;
    }

    // The caller is done with the path: the file was imported and removed. It is forgotten.
    pub fn finish(&self, path: &Path) {
        if let Ok(mut seen) = self.seen.lock() {
            seen.remove(path);
        }
    }

    // The path was listed but not submitted (the queue was busy): a later scan lists it again.
    pub fn release(&self, path: &Path) {
        if let Ok(mut seen) = self.seen.lock()
            && let Some(state) = seen.get_mut(path)
        {
            state.1 = false;
        }
    }

    // The import of the path failed. A file that is not a structure or is too big is renamed to
    // `<name>.failed`, or `<name>.failed-2` ... when that exists, and forgotten. A file that
    // could not be read stays and is not listed again until it changes.
    pub fn reject(&self, path: &Path, reason: &ImportFailure) {
        if matches!(reason, ImportFailure::Unreadable) {
            return;
        }
        for attempt in 1..=Self::MAX_FAILED_NAMES {
            let mut name = path.as_os_str().to_owned();
            name.push(if attempt == 1 { ".failed".to_string() } else { format!(".failed-{attempt}") });
            let target = PathBuf::from(name);
            if target.exists() {
                continue;
            }
            match std::fs::rename(path, &target) {
                Ok(()) => self.finish(path),
                Err(e) => curia::error!("failed inbox file could not be set aside", {
                    path: path.display().to_string(),
                    error: e.to_string(),
                }),
            }
            return;
        }
    }

    fn is_structure_name(path: &Path) -> bool {
        path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("mcstructure"))
    }
}
