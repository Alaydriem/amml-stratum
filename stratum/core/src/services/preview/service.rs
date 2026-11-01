use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender, TrySendError, bounded, select};

use super::worker::PreviewWorker;
use crate::config::PreviewConfig;
use crate::structs::preview::{PreviewFrame, PreviewRequest, PreviewSource, PreviewStop};

// Renders structure previews on its own thread, named `stratum-preview`, so a file read, a mesh build or a render
// never runs on the render thread. The UI submits a request when the view changes, takes the newest picture on a
// later frame and gives its pixel buffer back with `give_back`, so the thread draws the next picture into it. The
// thread sleeps on its channels while there is no request.
// Bound: `QUEUE` requests wait; a submit to a full queue is refused and the UI tries again on a later frame. The
// thread takes the newest waiting request and drops the older ones. One finished picture waits; a newer one
// replaces it. One new config waits; a newer one replaces it. `SPARES` given-back buffers wait; more are dropped.
pub struct PreviewService {
    requests: Sender<PreviewRequest>,
    results: Receiver<PreviewFrame>,
    spares: Sender<Vec<u8>>,
    halt: Sender<()>,
    configs: Sender<PreviewConfig>,
    // The other end of `configs`, to drop a config the thread has not applied when a newer one comes.
    unapplied: Receiver<PreviewConfig>,
    stop: PreviewStop,
    // Disconnects when the thread ends, so `shutdown` can wait for it with a time limit.
    ended: Receiver<()>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

impl PreviewService {
    // Requests that wait for the thread. The UI sends one request per view change, so a few cover a fast drag.
    pub const QUEUE: usize = 4;
    pub const SPARES: usize = 2;
    // The wait for the thread when the last handle drops without a `shutdown`.
    pub const DROP_BUDGET: Duration = Duration::from_millis(500);

    pub fn new(source: PreviewSource, config: &PreviewConfig) -> Self {
        let (request_tx, request_rx) = bounded::<PreviewRequest>(Self::QUEUE);
        let (result_tx, result_rx) = bounded::<PreviewFrame>(1);
        let (spare_tx, spare_rx) = bounded::<Vec<u8>>(Self::SPARES);
        let (halt_tx, halt_rx) = bounded::<()>(1);
        let (config_tx, config_rx) = bounded::<PreviewConfig>(1);
        let (ended_tx, ended_rx) = bounded::<()>(0);
        let stop = PreviewStop::new();
        let stale = result_rx.clone();
        let worker = PreviewWorker::new(source, config, stop.clone(), spare_rx, config_rx.clone());
        let thread_stop = stop.clone();
        let spawned = std::thread::Builder::new().name("stratum-preview".to_string()).spawn(move || {
            // dropped when the thread ends, a panic included
            let _ended = ended_tx;
            Self::run(worker, &request_rx, &halt_rx, &result_tx, &stale, &thread_stop);
        });
        let thread = match spawned {
            Ok(handle) => Some(handle),
            Err(e) => {
                curia::error!("preview thread could not start", { error: e.to_string() });
                None
            }
        };
        Self {
            requests: request_tx,
            results: result_rx,
            spares: spare_tx,
            halt: halt_tx,
            configs: config_tx,
            unapplied: config_rx,
            stop,
            ended: ended_rx,
            thread: Mutex::new(thread),
        }
    }

    pub fn new_shared(source: PreviewSource, config: &PreviewConfig) -> Arc<Self> {
        Arc::new(Self::new(source, config))
    }

    // Never blocks. A full queue, a stopped service or a thread that is gone gives the request back.
    pub fn submit(&self, request: PreviewRequest) -> Result<(), PreviewRequest> {
        if self.stop.is_stopped() {
            return Err(request);
        }
        match self.requests.try_send(request) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(request) | TrySendError::Disconnected(request)) => Err(request),
        }
    }

    // The newest finished picture, if one arrived since the last call. Never blocks.
    pub fn take(&self) -> Option<PreviewFrame> {
        let mut newest = None;
        while let Ok(frame) = self.results.try_recv() {
            if let Some(Ok(older)) = newest.replace(frame).map(|older| older.outcome) {
                self.give_back(older.rgba);
            }
        }
        newest
    }

    // Gives the thread new bounds, applied before it handles the next request: the texture cache and the mesh
    // cap take the new values (a mesh kept under another cap is built again). Never blocks; a newer config
    // replaces one the thread has not applied yet.
    pub fn reconfigure(&self, config: &PreviewConfig) {
        let mut config = config.clone();
        loop {
            match self.configs.try_send(config) {
                Ok(()) | Err(TrySendError::Disconnected(_)) => return,
                Err(TrySendError::Full(back)) => {
                    let _ = self.unapplied.try_recv();
                    config = back;
                }
            }
        }
    }

    // Hands a picture's pixel buffer back to the thread, which draws a later picture into it. Never blocks; a
    // buffer past `SPARES` is dropped.
    pub fn give_back(&self, rgba: Vec<u8>) {
        let _ = self.spares.try_send(rgba);
    }

    // Stops the thread: the mesh build or render in progress ends at its next stop check, and no new request is
    // taken. Waits at most `budget` for the thread to end and returns whether it did. A thread still running after
    // the budget is kept, so a later call waits for it again and returns its real state; once the thread has ended
    // a call returns true at once.
    pub fn shutdown(&self, budget: Duration) -> bool {
        self.stop.stop();
        let _ = self.halt.try_send(());
        let mut slot = match self.thread.lock() {
            Ok(slot) => slot,
            Err(poisoned) => poisoned.into_inner(),
        };
        if slot.is_none() {
            return true;
        }
        match self.ended.recv_timeout(budget) {
            Err(RecvTimeoutError::Disconnected) | Ok(()) => {
                if let Some(thread) = slot.take()
                    && thread.join().is_err()
                {
                    curia::error!("preview thread ended with a panic", {});
                }
                true
            }
            Err(RecvTimeoutError::Timeout) => {
                curia::warn!("preview thread still running after the shutdown budget", {
                    budget_ms: u64::try_from(budget.as_millis()).unwrap_or(u64::MAX),
                });
                false
            }
        }
    }

    fn run(
        mut worker: PreviewWorker,
        requests: &Receiver<PreviewRequest>,
        halt: &Receiver<()>,
        results: &Sender<PreviewFrame>,
        stale: &Receiver<PreviewFrame>,
        stop: &PreviewStop,
    ) {
        loop {
            let mut request = select! {
                recv(requests) -> request => match request {
                    Ok(request) => request,
                    Err(_) => return,
                },
                recv(halt) -> _ => return,
            };
            while let Ok(newer) = requests.try_recv() {
                request = newer;
            }
            if stop.is_stopped() {
                return;
            }
            let mut frame = worker.handle(&request);
            if stop.is_stopped() {
                return;
            }
            loop {
                match results.try_send(frame) {
                    Ok(()) | Err(TrySendError::Disconnected(_)) => break,
                    Err(TrySendError::Full(back)) => {
                        // the waiting picture is older than this one
                        if let Ok(older) = stale.try_recv() {
                            worker.reclaim(older);
                        }
                        frame = back;
                    }
                }
            }
        }
    }
}

impl Drop for PreviewService {
    fn drop(&mut self) {
        self.shutdown(Self::DROP_BUDGET);
    }
}
