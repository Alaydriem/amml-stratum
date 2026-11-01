use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, TrySendError, bounded};

use super::AtomicFile;
use super::store_state::StoreState;
use crate::errors::StoreError;

// Writes files on its own thread, named `stratum-store`, so the game thread never waits on a disk.
// Bound: at most `MAX_PENDING_PATHS` paths wait at once, one entry per path, and a newer write for a
// path replaces the older bytes. A submit for a new path past the bound is refused. Failed writes
// are kept (also one per path, same bound) until `retry_failed` queues them again.
pub struct StoreWriter {
    state: Arc<Mutex<StoreState>>,
    idle: Arc<Condvar>,
    paused: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
    // Set by `close`: the thread takes no new file, whatever is still waiting.
    abort: Arc<AtomicBool>,
    // Loop passes of the thread since `take_wakeups`, for telemetry.
    wakeups: Arc<AtomicU64>,
    tokens: Option<Sender<PathBuf>>,
    thread: Option<JoinHandle<()>>,
}

impl StoreWriter {
    pub const MAX_PENDING_PATHS: usize = 64;
    const POLL: Duration = Duration::from_millis(50);
    const PAUSE_POLL: Duration = Duration::from_millis(5);
    const CLOSE_POLL: Duration = Duration::from_millis(2);

    pub fn new() -> Self {
        let (token_tx, token_rx) = bounded::<PathBuf>(Self::MAX_PENDING_PATHS);
        let state = Arc::new(Mutex::new(StoreState::default()));
        let idle = Arc::new(Condvar::new());
        let paused = Arc::new(AtomicBool::new(false));
        let shutdown = Arc::new(AtomicBool::new(false));
        let abort = Arc::new(AtomicBool::new(false));
        let wakeups = Arc::new(AtomicU64::new(0));

        let thread_state = Arc::clone(&state);
        let thread_idle = Arc::clone(&idle);
        let thread_paused = Arc::clone(&paused);
        let thread_shutdown = Arc::clone(&shutdown);
        let thread_abort = Arc::clone(&abort);
        let thread_wakeups = Arc::clone(&wakeups);
        let spawned = std::thread::Builder::new().name("stratum-store".to_string()).spawn(move || {
            Self::run(
                &token_rx,
                &thread_state,
                &thread_idle,
                &thread_paused,
                &thread_shutdown,
                &thread_abort,
                &thread_wakeups,
            );
        });
        let thread = match spawned {
            Ok(handle) => Some(handle),
            Err(e) => {
                curia::error!("store thread could not start", { error: e.to_string() });
                None
            }
        };
        Self { state, idle, paused, shutdown, abort, wakeups, tokens: Some(token_tx), thread }
    }

    // Never waits for the disk. Replaces older bytes waiting for the same path.
    pub fn submit(&self, path: PathBuf, bytes: Vec<u8>) -> Result<(), StoreError> {
        let busy = || StoreError::Io("the write queue is full".to_string());
        let Some(tokens) = &self.tokens else {
            return Err(busy());
        };
        let mut state = self.state.lock().map_err(|_| busy())?;
        if !state.pending.contains_key(&path) && state.pending.len() >= Self::MAX_PENDING_PATHS {
            return Err(busy());
        }
        state.failed.remove(&path);
        if state.pending.insert(path.clone(), Arc::new(bytes)).is_some() {
            return Ok(());
        }
        match tokens.try_send(path.clone()) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => {
                state.pending.remove(&path);
                Err(busy())
            }
        }
    }

    // Queues the failed writes again, as many as fit under the bound. A path with newer bytes keeps
    // them, and a write that does not fit stays failed.
    pub fn retry_failed(&self) {
        let Some(tokens) = &self.tokens else {
            return;
        };
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        let mut full = false;
        for (path, bytes) in std::mem::take(&mut state.failed) {
            if state.pending.contains_key(&path) {
                continue;
            }
            if full || state.pending.len() >= Self::MAX_PENDING_PATHS {
                full = true;
                state.failed.insert(path, bytes);
                continue;
            }
            state.pending.insert(path.clone(), bytes);
            if tokens.try_send(path.clone()).is_err() {
                full = true;
                if let Some(bytes) = state.pending.remove(&path) {
                    state.failed.insert(path, bytes);
                }
            }
        }
    }

    // Waits up to `timeout` until nothing is pending or being written. For shutdown and tests.
    pub fn flush(&self, timeout: Duration) -> bool {
        let Ok(state) = self.state.lock() else {
            return false;
        };
        match self.idle.wait_timeout_while(state, timeout, |s| !s.pending.is_empty() || !s.writing.is_empty()) {
            Ok((_, result)) => !result.timed_out(),
            Err(_) => false,
        }
    }

    // Stops the thread for shutdown: it finishes the write in progress and takes no other file. Waits
    // up to `wait` for it; a thread still running after that is left behind. Files still waiting
    // are logged as lost. Returns false when the thread was left behind.
    pub fn close(&mut self, wait: Duration) -> bool {
        self.report_unsaved();
        self.abort.store(true, Ordering::SeqCst);
        self.tokens = None;
        let Some(thread) = self.thread.take() else {
            return true;
        };
        let start = Instant::now();
        while !thread.is_finished() {
            if start.elapsed() >= wait {
                curia::error!("store thread did not stop in time and was left running", {});
                return false;
            }
            std::thread::sleep(Self::CLOSE_POLL);
        }
        if thread.join().is_err() {
            curia::error!("store thread panicked", {});
        }
        true
    }

    // One error line that names the files that will not reach the disk.
    fn report_unsaved(&self) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        let mut paths: Vec<String> =
            state.pending.keys().chain(state.failed.keys()).map(|path| path.display().to_string()).collect();
        if paths.is_empty() {
            return;
        }
        paths.sort();
        state.failure_logs += 1;
        curia::error!("files were not saved", { paths: paths.join(", ") });
    }

    // The thread finishes its current file, then holds the next one until `resume`.
    pub fn pause(&self) {
        self.paused.store(true, Ordering::SeqCst);
    }

    pub fn resume(&self) {
        self.paused.store(false, Ordering::SeqCst);
    }

    // The newest bytes not yet on disk for `path`: waiting, being written, or failed.
    pub fn latest(&self, path: &Path) -> Option<Arc<Vec<u8>>> {
        let state = self.state.lock().ok()?;
        state.pending.get(path).or_else(|| state.writing.get(path)).or_else(|| state.failed.get(path)).cloned()
    }

    // True when the newest bytes submitted for `path` are on disk: none wait, are being written or
    // failed. False for the rest of the session once a failed write was dropped, because the
    // dropped one may have been for `path`.
    pub fn saved(&self, path: &Path) -> bool {
        self.state.lock().is_ok_and(|state| {
            state.lost == 0
                && !state.pending.contains_key(path)
                && !state.writing.contains_key(path)
                && !state.failed.contains_key(path)
        })
    }

    // Loop passes of the store thread since the last call; the count restarts at zero.
    pub fn take_wakeups(&self) -> u64 {
        self.wakeups.swap(0, Ordering::Relaxed)
    }

    // Paths waiting to be written.
    pub fn pending(&self) -> usize {
        self.state.lock().map_or(0, |state| state.pending.len())
    }

    // Writes that failed and wait for `retry_failed`.
    pub fn failed(&self) -> usize {
        self.state.lock().map_or(0, |state| state.failed.len())
    }

    // Failed writes since the start.
    pub fn failures(&self) -> u64 {
        self.state.lock().map_or(0, |state| state.failures)
    }

    // True from a failed write until the next successful one.
    pub fn failing(&self) -> bool {
        self.state.lock().is_ok_and(|state| state.in_failure_streak)
    }

    // Log lines written for failures: one per failure streak.
    pub fn failure_logs(&self) -> u64 {
        self.state.lock().map_or(0, |state| state.failure_logs)
    }

    fn run(
        tokens: &Receiver<PathBuf>,
        state: &Mutex<StoreState>,
        idle: &Condvar,
        paused: &AtomicBool,
        shutdown: &AtomicBool,
        abort: &AtomicBool,
        wakeups: &AtomicU64,
    ) {
        loop {
            wakeups.fetch_add(1, Ordering::Relaxed);
            let path = match tokens.recv_timeout(Self::POLL) {
                Ok(path) => path,
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => break,
            };
            while paused.load(Ordering::SeqCst) && !shutdown.load(Ordering::SeqCst) && !abort.load(Ordering::SeqCst) {
                std::thread::sleep(Self::PAUSE_POLL);
            }
            if abort.load(Ordering::SeqCst) {
                break;
            }
            let bytes = {
                let Ok(mut guard) = state.lock() else {
                    break;
                };
                let Some(bytes) = guard.pending.remove(&path) else {
                    idle.notify_all();
                    continue;
                };
                guard.writing.insert(path.clone(), Arc::clone(&bytes));
                bytes
            };
            let written = Self::write(&path, &bytes);
            let Ok(mut guard) = state.lock() else {
                break;
            };
            guard.writing.remove(&path);
            match written {
                Ok(()) => guard.in_failure_streak = false,
                Err(error) => {
                    guard.failures += 1;
                    if !guard.in_failure_streak {
                        guard.in_failure_streak = true;
                        guard.failure_logs += 1;
                        curia::error!("file could not be saved", {
                            path: path.display().to_string(),
                            error: error.to_string(),
                        });
                    }
                    if !guard.pending.contains_key(&path) {
                        if guard.failed.contains_key(&path) || guard.failed.len() < Self::MAX_PENDING_PATHS {
                            guard.failed.insert(path, bytes);
                        } else {
                            guard.lost += 1;
                        }
                    }
                }
            }
            idle.notify_all();
        }
    }

    fn write(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        AtomicFile::write(path, bytes)
    }
}

impl Default for StoreWriter {
    fn default() -> Self {
        Self::new()
    }
}

// Writes what is still pending, then joins the thread.
impl Drop for StoreWriter {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        self.tokens = None;
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            curia::error!("store thread panicked", {});
        }
    }
}
