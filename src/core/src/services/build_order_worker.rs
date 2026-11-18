use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use crossbeam_channel::{Receiver, RecvTimeoutError, SendTimeoutError, Sender, TrySendError, bounded};

use std::collections::HashMap;

use super::{BuildOrder, GhostAssembler};
use crate::structs::build::{
    AssembleRequest, AssembleResult, BuildAnswer, BuildJob, BuildPass, CountRequest, CountResult, OrderRequest,
    OrderResult,
};
use crate::structs::library::Library;
use crate::structs::marker::{Marker, MarkerId, MarkerTarget, MarkerView};

// Does the work of a build on its own thread, named `stratum-build`, so the game thread never does it: places
// the structure in the world, counts the blocks to place, and works out the order of a layer. The session asks for
// one job at a time, and for one layer ahead.
// Bound: `QUEUE` requests and `QUEUE` results wait in their channels. A full request channel refuses new
// requests, and the worker blocks on a full result channel until it is drained.
pub struct BuildOrderWorker {
    requests: Option<Sender<BuildJob>>,
    results: Receiver<BuildAnswer>,
    // A worker with no thread answers on the thread that submits, into this channel.
    inline: Option<Sender<BuildAnswer>>,
    shutdown: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    // Keeps the request channel open for a worker with no thread; `None` otherwise.
    _parked: Option<Receiver<BuildJob>>,
}

impl BuildOrderWorker {
    pub const QUEUE: usize = 4;
    const POLL: Duration = Duration::from_millis(50);

    pub fn new() -> Self {
        let (request_tx, request_rx) = bounded::<BuildJob>(Self::QUEUE);
        let (result_tx, result_rx) = bounded::<BuildAnswer>(Self::QUEUE);
        let shutdown = Arc::new(AtomicBool::new(false));
        let thread_shutdown = Arc::clone(&shutdown);
        let spawned = std::thread::Builder::new().name("stratum-build".to_string()).spawn(move || {
            Self::run(&request_rx, &result_tx, &thread_shutdown);
        });
        let thread = match spawned {
            Ok(handle) => Some(handle),
            Err(e) => {
                curia::error!("build order thread could not start", { error: e.to_string() });
                None
            }
        };
        Self { requests: Some(request_tx), results: result_rx, inline: None, shutdown, thread, _parked: None }
    }

    // A worker with no thread: accepted requests stay queued, so the request bound is reached after exactly
    // `QUEUE` submits.
    pub fn without_thread() -> Self {
        let (request_tx, request_rx) = bounded::<BuildJob>(Self::QUEUE);
        let (_result_tx, result_rx) = bounded::<BuildAnswer>(Self::QUEUE);
        Self {
            requests: Some(request_tx),
            results: result_rx,
            inline: None,
            shutdown: Arc::new(AtomicBool::new(false)),
            thread: None,
            _parked: Some(request_rx),
        }
    }

    // A worker with no thread that works out each request when it is submitted.
    pub fn inline() -> Self {
        let (result_tx, result_rx) = bounded::<BuildAnswer>(Self::QUEUE);
        Self {
            requests: None,
            results: result_rx,
            inline: Some(result_tx),
            shutdown: Arc::new(AtomicBool::new(false)),
            thread: None,
            _parked: None,
        }
    }

    // The order of the layer the request names.
    pub fn work(request: OrderRequest) -> OrderResult {
        let steps = match request.pass {
            BuildPass::Main => BuildOrder::layer(&request.piece, request.layer, &request.skip),
            BuildPass::Hanging => BuildOrder::hanging_layer(&request.piece, request.layer, &request.skip),
            BuildPass::Fluid => BuildOrder::fluid_layer(&request.piece, request.layer, &request.skip),
        };
        OrderResult { serial: request.serial, pass: request.pass, layer: request.layer, steps }
    }

    // The structure placed in the world, at its origin and turned by its rotation.
    pub fn assemble(request: AssembleRequest) -> AssembleResult {
        let marker = Marker {
            id: MarkerId(0),
            name: String::new(),
            origin: request.origin,
            rotation: request.rotation,
            target: MarkerTarget::Structure(request.id),
            view: MarkerView::default(),
        };
        let loaded = HashMap::from([(request.id, request.structure)]);
        let build = GhostAssembler::assemble(&marker, &Library::new(), &loaded, 0);
        AssembleResult { serial: request.serial, build }
    }

    // The blocks the piece places.
    pub fn count(request: CountRequest) -> CountResult {
        CountResult { serial: request.serial, total: BuildOrder::count(&request.piece, &request.skip) }
    }

    // What the job asks for.
    pub fn perform(job: BuildJob) -> BuildAnswer {
        match job {
            BuildJob::Assemble(request) => BuildAnswer::Assemble(Self::assemble(request)),
            BuildJob::Count(request) => BuildAnswer::Count(Self::count(request)),
            BuildJob::Order(request) => BuildAnswer::Order(Self::work(request)),
        }
    }

    // Never blocks. A full queue, or a worker that is gone, gives the job back.
    pub fn submit(&self, job: BuildJob) -> Result<(), BuildJob> {
        if let Some(results) = &self.inline {
            if results.is_full() {
                return Err(job);
            }
            let _ = results.try_send(Self::perform(job));
            return Ok(());
        }
        let Some(requests) = &self.requests else {
            return Err(job);
        };
        match requests.try_send(job) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(job) | TrySendError::Disconnected(job)) => Err(job),
        }
    }

    // Moves finished jobs into `out` without waiting.
    pub fn drain(&self, out: &mut Vec<BuildAnswer>) {
        while let Ok(result) = self.results.try_recv() {
            out.push(result);
        }
    }

    fn run(requests: &Receiver<BuildJob>, results: &Sender<BuildAnswer>, shutdown: &AtomicBool) {
        while !shutdown.load(Ordering::SeqCst) {
            let request = match requests.recv_timeout(Self::POLL) {
                Ok(job) => job,
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => break,
            };
            let mut result = Self::perform(request);
            loop {
                match results.send_timeout(result, Self::POLL) {
                    Ok(()) | Err(SendTimeoutError::Disconnected(_)) => break,
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
}

impl Default for BuildOrderWorker {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for BuildOrderWorker {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::SeqCst);
        self.requests = None;
        if let Some(thread) = self.thread.take()
            && thread.join().is_err()
        {
            curia::error!("build order thread panicked", {});
        }
    }
}
