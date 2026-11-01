use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, SyncSender};
use std::thread::JoinHandle;
use std::time::Duration;

use crate::services::ThreadWindows;
use crate::traits::DialogRunner;
use crate::types::PickResult;

// runs the structure file picker on one worker thread at a time and hands its answer to the game thread. The core
// decides when to pick and what to do with the files; this only moves the request and the answer.
// Bound: one worker and at most one answer waiting, in a channel of capacity 1
pub struct PickerPump<R: DialogRunner> {
    runner: Arc<R>,
    worker: Option<JoinHandle<()>>,
    // the OS id of the worker thread while its dialog may be open, 0 otherwise
    worker_thread: Arc<AtomicU32>,
    // set by stop: a worker that has not shown its dialog yet answers as a cancel
    stop_requested: Arc<AtomicBool>,
    sender: SyncSender<PickResult>,
    answers: Receiver<PickResult>,
    // a picker is open or its answer is not yet delivered
    busy: bool,
}

impl<R: DialogRunner> PickerPump<R> {
    // how long stop waits for the dialog to close before it joins anyway: CLOSE_ATTEMPTS times CLOSE_WAIT
    pub const CLOSE_ATTEMPTS: u32 = 100;
    pub const CLOSE_WAIT: Duration = Duration::from_millis(50);

    pub fn new(runner: R) -> Self {
        let (sender, answers) = std::sync::mpsc::sync_channel(1);
        Self {
            runner: Arc::new(runner),
            worker: None,
            worker_thread: Arc::new(AtomicU32::new(0)),
            stop_requested: Arc::new(AtomicBool::new(false)),
            sender,
            answers,
            busy: false,
        }
    }

    // on the game thread: opens the picker on a worker thread; ignored while one is open or not yet delivered
    pub fn request(&mut self) {
        if std::mem::replace(&mut self.busy, true) {
            return;
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        self.stop_requested.store(false, Ordering::SeqCst);
        let runner = Arc::clone(&self.runner);
        let thread = Arc::clone(&self.worker_thread);
        let stop = Arc::clone(&self.stop_requested);
        let sender = self.sender.clone();
        let spawned = std::thread::Builder::new().name("stratum-picker".to_string()).spawn(move || {
            thread.store(ThreadWindows::current(), Ordering::SeqCst);
            let result = if stop.load(Ordering::SeqCst) { PickResult::cancelled() } else { runner.pick() };
            thread.store(0, Ordering::SeqCst);
            let _ = sender.try_send(result);
        });
        match spawned {
            Ok(worker) => self.worker = Some(worker),
            Err(error) => {
                self.busy = false;
                curia::warn!("structure picker thread not started", { error: error.to_string() });
            }
        }
    }

    // on the game thread, once per tick: a finished pick, with its worker joined; a failure, dropped items and paths
    // that are not valid UTF-16 are logged. A cancel gives no paths
    pub fn deliver(&mut self) -> Option<PickResult> {
        let answer = self.answers.try_recv().ok()?;
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        self.busy = false;
        if answer.failed() {
            curia::warn!("structure picker failed", { hresult: format!("0x{:08X}", answer.status as u32) });
        }
        if answer.dropped > 0 {
            curia::warn!("structure picker dropped items", { count: answer.dropped });
        }
        if answer.invalid > 0 {
            curia::warn!("structure picker skipped paths that are not valid UTF-16", { count: answer.invalid });
        }
        Some(answer)
    }

    // on disable: closes an open dialog as a cancel, joins the worker and drops an undelivered answer. The close is
    // retried for a bounded time and logged when it runs out; the join after it is unbounded, because a worker left
    // running could outlive this module's code
    pub fn stop(&mut self) {
        if let Some(worker) = self.worker.take() {
            self.stop_requested.store(true, Ordering::SeqCst);
            // the dialog is modal on the worker thread, so it is closed as a cancel before the join
            for _ in 0..Self::CLOSE_ATTEMPTS {
                if worker.is_finished() {
                    break;
                }
                let thread = self.worker_thread.load(Ordering::SeqCst);
                if thread != 0 {
                    ThreadWindows::post_close(thread);
                }
                std::thread::sleep(Self::CLOSE_WAIT);
            }
            if !worker.is_finished() {
                curia::warn!("structure picker did not close; waiting for it", { attempts: Self::CLOSE_ATTEMPTS });
            }
            let _ = worker.join();
        }
        while self.answers.try_recv().is_ok() {}
        self.busy = false;
    }

    // a picker is open or its answer is not yet delivered
    pub fn is_busy(&self) -> bool {
        self.busy
    }

    // a worker thread is started and not yet joined
    pub fn has_worker(&self) -> bool {
        self.worker.is_some()
    }
}
