use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use stratum_core::services::ImportWorker;
use stratum_core::structs::import::{ImportRequest, ImportResult};

// Helpers for import tests: files on disk and waiting for worker results.
pub struct ImportHarness;

impl ImportHarness {
    const DEADLINE: Duration = Duration::from_secs(60);

    pub fn write(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    // Waits on the result channel until `count` results arrived.
    pub fn collect(worker: &ImportWorker, count: usize) -> Vec<ImportResult> {
        let start = Instant::now();
        let mut results = Vec::new();
        while results.len() < count {
            assert!(start.elapsed() < Self::DEADLINE, "import results did not arrive");
            worker.drain_blocking(&mut results, Duration::from_millis(500));
        }
        results
    }

    pub fn request(source: PathBuf) -> ImportRequest {
        ImportRequest { source }
    }

    // The last path part of each path, sorted.
    pub fn file_names(paths: &[PathBuf]) -> Vec<String> {
        let mut names: Vec<String> =
            paths.iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
        names.sort();
        names
    }

    // File names in the structures folder, sorted.
    pub fn stored_files(store: &Path) -> Vec<String> {
        let Ok(entries) = std::fs::read_dir(store.join("structures")) else {
            return Vec::new();
        };
        let mut names: Vec<String> = entries.map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        names.sort();
        names
    }
}
