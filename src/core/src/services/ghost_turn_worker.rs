use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError, SendTimeoutError, Sender, TrySendError, bounded};

use super::GhostAssembler;
use crate::structs::ghost::{TurnDone, TurnJob};

// Turns build structures on its own thread, named `stratum-ghost-turn`, so a turn press never
// copies a build on the game thread.
// Bound: `QUEUE` requests and `QUEUE` results wait in their channels. A full request channel
// refuses new requests, and the worker blocks on a full result channel until it is drained.
// `shutdown` stops a turn between X slices of a piece, so the join never waits for a whole turn.
pub struct GhostTurnWorker {
    requests: Option<Sender<TurnJob>>,
    results: Receiver<TurnDone>,
    shutdown: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl GhostTurnWorker {
    pub const QUEUE: usize = 1;
    const POLL: Duration = Duration::from_millis(50);

    pub fn new() -> Self {
        let (request_tx, request_rx) = bounded::<TurnJob>(Self::QUEUE);
        let (result_tx, result_rx) = bounded::<TurnDone>(Self::QUEUE);
        let shutdown = Arc::new(AtomicBool::new(false));
        let thread_shutdown = Arc::clone(&shutdown);
        let spawned = std::thread::Builder::new()
            .name("stratum-ghost-turn".to_string())
            .spawn(move || Self::run(&request_rx, &result_tx, &thread_shutdown));
        let thread = match spawned {
            Ok(handle) => Some(handle),
            Err(e) => {
                curia::error!("ghost turn thread could not start", { error: e.to_string() });
                None
            }
        };
        Self { requests: Some(request_tx), results: result_rx, shutdown, thread }
    }

    // False when the thread could not start, has stopped or was shut down: the caller then turns
    // on its own thread.
    pub fn alive(&self) -> bool {
        self.thread.as_ref().is_some_and(|thread| !thread.is_finished())
    }

    // Stops the thread and joins it. A turn that runs is given up. Calling it again does nothing.
    pub fn shutdown(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        self.requests = None;
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            curia::error!("ghost turn thread panicked", {});
        }
    }

    // Never blocks. A full queue, or a worker that is gone, gives the request back.
    pub fn submit(&self, job: TurnJob) -> Result<(), TurnJob> {
        let Some(requests) = &self.requests else {
            return Err(job);
        };
        match requests.try_send(job) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(job) | TrySendError::Disconnected(job)) => Err(job),
        }
    }

    // A finished turn, without waiting.
    pub fn take(&self) -> Option<TurnDone> {
        self.results.try_recv().ok()
    }

    fn run(requests: &Receiver<TurnJob>, results: &Sender<TurnDone>, shutdown: &AtomicBool) {
        while !shutdown.load(Ordering::SeqCst) {
            let job = match requests.recv_timeout(Self::POLL) {
                Ok(job) => job,
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => break,
            };
            let mut turned = Vec::with_capacity(job.sources.len());
            for structure in &job.sources {
                match GhostAssembler::turn_unless(structure, job.rotation, shutdown) {
                    Some(copy) => turned.push(Arc::new(copy)),
                    None => return,
                }
            }
            let mut done = TurnDone { ticket: job.ticket, rotation: job.rotation, sources: job.sources, turned };
            loop {
                match results.send_timeout(done, Self::POLL) {
                    Ok(()) | Err(SendTimeoutError::Disconnected(_)) => break,
                    Err(SendTimeoutError::Timeout(back)) => {
                        if shutdown.load(Ordering::SeqCst) {
                            return;
                        }
                        done = back;
                    }
                }
            }
        }
    }
}

impl Default for GhostTurnWorker {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for GhostTurnWorker {
    fn drop(&mut self) {
        self.shutdown();
    }
}
