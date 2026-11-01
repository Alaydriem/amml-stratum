use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

// What `StoreWriter` and its thread share, behind one lock.
#[derive(Default)]
pub(crate) struct StoreState {
    // The newest bytes waiting for each path.
    pub(crate) pending: HashMap<PathBuf, Arc<Vec<u8>>>,
    // The bytes of writes that failed and were not replaced by newer bytes.
    pub(crate) failed: HashMap<PathBuf, Arc<Vec<u8>>>,
    // The bytes being written now.
    pub(crate) writing: HashMap<PathBuf, Arc<Vec<u8>>>,
    pub(crate) failures: u64,
    pub(crate) failure_logs: u64,
    // True after a failure until the next success, so a streak logs once.
    pub(crate) in_failure_streak: bool,
    // Failed writes whose bytes were dropped because the failed list was full.
    pub(crate) lost: u64,
}
