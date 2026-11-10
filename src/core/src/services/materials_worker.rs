use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError, SendTimeoutError, Sender, TrySendError, bounded};

use super::StructureParser;
use crate::errors::StructureError;
use crate::structs::material::{StructureLoad, StructureLoaded};

// Reads and parses stored structures on its own thread, named `stratum-materials`, so counting
// materials never reads a file on the game thread.
// Bound: `QUEUE` requests and `QUEUE` results wait in their channels. A full request channel
// refuses new requests, and the worker blocks on a full result channel until it is drained.
pub struct MaterialsWorker {
    requests: Option<Sender<StructureLoad>>,
    results: Receiver<StructureLoaded>,
    shutdown: Arc<AtomicBool>,
    // Loop passes of the thread since `take_wakeups`, for telemetry.
    wakeups: Arc<AtomicU64>,
    thread: Option<JoinHandle<()>>,
    // Keeps the request channel open for a worker with no thread; `None` otherwise.
    _parked: Option<Receiver<StructureLoad>>,
}

impl MaterialsWorker {
    pub const QUEUE: usize = 16;
    const POLL: Duration = Duration::from_millis(50);

    pub fn new() -> Self {
        let (request_tx, request_rx) = bounded::<StructureLoad>(Self::QUEUE);
        let (result_tx, result_rx) = bounded::<StructureLoaded>(Self::QUEUE);
        let shutdown = Arc::new(AtomicBool::new(false));
        let wakeups = Arc::new(AtomicU64::new(0));
        let thread_shutdown = Arc::clone(&shutdown);
        let thread_wakeups = Arc::clone(&wakeups);
        let spawned = std::thread::Builder::new().name("stratum-materials".to_string()).spawn(move || {
            Self::run(&request_rx, &result_tx, &thread_shutdown, &thread_wakeups);
        });
        let thread = match spawned {
            Ok(handle) => Some(handle),
            Err(e) => {
                curia::error!("materials thread could not start", { error: e.to_string() });
                None
            }
        };
        Self { requests: Some(request_tx), results: result_rx, shutdown, wakeups, thread, _parked: None }
    }

    // A worker with no consumer thread: accepted requests stay queued, so the request bound is
    // reached after exactly `QUEUE` submits.
    pub fn without_thread() -> Self {
        let (request_tx, request_rx) = bounded::<StructureLoad>(Self::QUEUE);
        let (_result_tx, result_rx) = bounded::<StructureLoaded>(Self::QUEUE);
        Self {
            requests: Some(request_tx),
            results: result_rx,
            shutdown: Arc::new(AtomicBool::new(false)),
            wakeups: Arc::new(AtomicU64::new(0)),
            thread: None,
            _parked: Some(request_rx),
        }
    }

    // Never blocks. A full queue, or a worker that is gone, gives the request back.
    pub fn submit(&self, load: StructureLoad) -> Result<(), StructureLoad> {
        let Some(requests) = &self.requests else {
            return Err(load);
        };
        match requests.try_send(load) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(load) | TrySendError::Disconnected(load)) => Err(load),
        }
    }

    // Moves finished loads into `out` without waiting.
    pub fn drain(&self, out: &mut Vec<StructureLoaded>) {
        while let Ok(loaded) = self.results.try_recv() {
            out.push(loaded);
        }
    }

    // Loop passes of the worker thread since the last call; the count restarts at zero.
    pub fn take_wakeups(&self) -> u64 {
        self.wakeups.swap(0, Ordering::Relaxed)
    }

    fn run(
        requests: &Receiver<StructureLoad>,
        results: &Sender<StructureLoaded>,
        shutdown: &AtomicBool,
        wakeups: &AtomicU64,
    ) {
        while !shutdown.load(Ordering::SeqCst) {
            wakeups.fetch_add(1, Ordering::Relaxed);
            let load = match requests.recv_timeout(Self::POLL) {
                Ok(load) => load,
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => break,
            };
            let outcome = std::fs::read(&load.path)
                .map_err(|e| StructureError::Unreadable(e.to_string()))
                .and_then(|bytes| StructureParser::parse(&bytes))
                .map(Arc::new);
            let mut loaded = StructureLoaded { id: load.id, outcome };
            loop {
                match results.send_timeout(loaded, Self::POLL) {
                    Ok(()) | Err(SendTimeoutError::Disconnected(_)) => break,
                    Err(SendTimeoutError::Timeout(back)) => {
                        if shutdown.load(Ordering::SeqCst) {
                            return;
                        }
                        loaded = back;
                    }
                }
            }
        }
    }
}

impl Default for MaterialsWorker {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for MaterialsWorker {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        self.requests = None;
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            curia::error!("materials thread panicked", {});
        }
    }
}
