use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Weak};
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError, SendTimeoutError, Sender, TrySendError, bounded};

use super::structure_importer::StructureImporter;
use crate::config::ImportConfig;
use crate::errors::ImportBusy;
use crate::structs::import::{ImportRequest, ImportResult};

// Imports structure files on its own thread, named `stratum-import`.
// Bound: `config.queue` requests wait in the request channel and `config.queue` results wait in
// the result channel. The worker holds at most one more request while paused, and blocks on a
// full result channel until the caller drains it. A full request channel refuses new requests.
pub struct ImportWorker {
    store_dir: PathBuf,
    requests: Option<Sender<ImportRequest>>,
    results: Receiver<ImportResult>,
    paused: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
    // Loop passes of the thread since `take_wakeups`, for telemetry.
    wakeups: Arc<AtomicU64>,
    // Dies when the worker thread ends.
    liveness: Weak<()>,
    thread: Option<JoinHandle<()>>,
}

impl ImportWorker {
    const POLL: Duration = Duration::from_millis(50);
    const PAUSE_POLL: Duration = Duration::from_millis(5);

    pub fn new(store_dir: PathBuf, config: ImportConfig) -> Self {
        let capacity = config.queue.clamp(1, ImportConfig::MAX_QUEUE);
        let (request_tx, request_rx) = bounded::<ImportRequest>(capacity);
        let (result_tx, result_rx) = bounded::<ImportResult>(capacity);
        let paused = Arc::new(AtomicBool::new(false));
        let shutdown = Arc::new(AtomicBool::new(false));
        let wakeups = Arc::new(AtomicU64::new(0));
        let token = Arc::new(());
        let liveness = Arc::downgrade(&token);

        let importer = StructureImporter::new(store_dir.clone(), &config);
        let thread_paused = Arc::clone(&paused);
        let thread_shutdown = Arc::clone(&shutdown);
        let thread_wakeups = Arc::clone(&wakeups);
        let spawned = std::thread::Builder::new().name("stratum-import".to_string()).spawn(move || {
            let _token = token;
            Self::run(&importer, &request_rx, &result_tx, &thread_paused, &thread_shutdown, &thread_wakeups);
        });
        let thread = match spawned {
            Ok(handle) => Some(handle),
            Err(e) => {
                curia::error!("import thread could not start", { error: e.to_string() });
                None
            }
        };
        Self { store_dir, requests: Some(request_tx), results: result_rx, paused, shutdown, wakeups, liveness, thread }
    }

    // Never blocks. A full queue, or a worker that is gone, gives `ImportBusy`.
    pub fn submit(&self, request: ImportRequest) -> Result<(), ImportBusy> {
        let Some(requests) = &self.requests else {
            return Err(ImportBusy);
        };
        match requests.try_send(request) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => Err(ImportBusy),
        }
    }

    // Moves finished results into `out` without waiting.
    pub fn drain(&self, out: &mut Vec<ImportResult>) {
        while let Ok(result) = self.results.try_recv() {
            out.push(result);
        }
    }

    // Waits up to `timeout` for a first result, then drains the rest. For threads other than the game thread.
    pub fn drain_blocking(&self, out: &mut Vec<ImportResult>, timeout: Duration) {
        if let Ok(first) = self.results.recv_timeout(timeout) {
            out.push(first);
        }
        self.drain(out);
    }

    // The worker finishes its current file, then holds the next request until `resume`.
    pub fn pause(&self) {
        self.paused.store(true, Ordering::SeqCst);
    }

    pub fn resume(&self) {
        self.paused.store(false, Ordering::SeqCst);
    }

    // Removes a stored copy the library refused. Names with a path part are ignored.
    pub fn discard_stored(&self, stored_file: &str) -> std::io::Result<()> {
        let plain = !stored_file.is_empty() && !stored_file.contains(['/', '\\', ':']) && stored_file != "..";
        if !plain {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "not a plain file name"));
        }
        std::fs::remove_file(self.store_dir.join("structures").join(stored_file))
    }

    // Loop passes of the worker thread since the last call; the count restarts at zero.
    pub fn take_wakeups(&self) -> u64 {
        self.wakeups.swap(0, Ordering::Relaxed)
    }

    // Upgrades while the worker thread runs.
    pub fn liveness(&self) -> Weak<()> {
        Weak::clone(&self.liveness)
    }

    fn run(
        importer: &StructureImporter,
        requests: &Receiver<ImportRequest>,
        results: &Sender<ImportResult>,
        paused: &AtomicBool,
        shutdown: &AtomicBool,
        wakeups: &AtomicU64,
    ) {
        while !shutdown.load(Ordering::SeqCst) {
            wakeups.fetch_add(1, Ordering::Relaxed);
            let request = match requests.recv_timeout(Self::POLL) {
                Ok(request) => request,
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => break,
            };
            while paused.load(Ordering::SeqCst) && !shutdown.load(Ordering::SeqCst) {
                std::thread::sleep(Self::PAUSE_POLL);
            }
            if shutdown.load(Ordering::SeqCst) {
                break;
            }
            Self::deliver(results, shutdown, importer.import(&request));
        }
    }

    fn deliver(results: &Sender<ImportResult>, shutdown: &AtomicBool, mut result: ImportResult) {
        loop {
            match results.send_timeout(result, Self::POLL) {
                Ok(()) | Err(SendTimeoutError::Disconnected(_)) => return,
                Err(SendTimeoutError::Timeout(back)) => {
                    if shutdown.load(Ordering::SeqCst) {
                        return;
                    }
                    result = back;
                }
            }
        }
    }
}

impl Drop for ImportWorker {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        self.requests = None;
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            curia::error!("import thread panicked", {});
        }
    }
}
