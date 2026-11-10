use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use super::FrameMeter;
use crossbeam_channel::{Receiver, Sender};
use stratum_core::config::PreviewConfig;
use stratum_core::services::preview::PreviewService;
use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::hud::HudState;
use stratum_core::structs::snapshot::LibrarySnapshot;

// What the game thread and the render thread share. The tick writes the snapshot and the open state; the frame
// reads them and sends the UI's actions back through a bounded queue. Neither side waits for the other: the
// snapshot lock is held only to swap or clone one Arc, and the frame takes it with try_lock.
// Bound: at most `ACTION_CAPACITY` actions wait for a tick; a frame's actions past that are dropped and logged.
pub struct SharedUi {
    snapshot: Mutex<Arc<LibrarySnapshot>>,
    // The HUD card state while the Hud lease is held, None otherwise. The lock is held only to copy it.
    hud: Mutex<Option<HudState>>,
    actions: Sender<LibraryAction>,
    inbox: Receiver<LibraryAction>,
    open: AtomicBool,
    // Set by the frame when the player closes the library; the tick takes it and gives the leases back.
    close_requested: AtomicBool,
    // Counts the opens, so the frame starts each one with fresh UI state.
    opens: AtomicU64,
    meter: FrameMeter,
    // The 3D structure preview the library card shows; None shows the size box instead.
    preview: Option<Arc<PreviewService>>,
}

impl SharedUi {
    pub const ACTION_CAPACITY: usize = 256;

    // `preview` draws the 3D preview in the library card; None shows the size box instead.
    pub fn new(preview: Option<Arc<PreviewService>>) -> Self {
        let (actions, inbox) = crossbeam_channel::bounded(Self::ACTION_CAPACITY);
        Self {
            snapshot: Mutex::new(Arc::new(LibrarySnapshot::empty())),
            hud: Mutex::new(None),
            actions,
            inbox,
            open: AtomicBool::new(false),
            close_requested: AtomicBool::new(false),
            opens: AtomicU64::new(0),
            meter: FrameMeter::new(),
            preview,
        }
    }

    pub fn new_shared(preview: Option<Arc<PreviewService>>) -> Arc<Self> {
        Arc::new(Self::new(preview))
    }

    pub fn preview(&self) -> Option<&Arc<PreviewService>> {
        self.preview.as_ref()
    }

    // Game thread: hands new preview bounds to the preview thread; nothing happens without a preview.
    pub fn reconfigure_preview(&self, config: &PreviewConfig) {
        if let Some(preview) = &self.preview {
            preview.reconfigure(config);
        }
    }

    // The telemetry the frame fills and the tick reads.
    pub fn meter(&self) -> &FrameMeter {
        &self.meter
    }

    // Game thread: the library shows from the next frame.
    pub fn open(&self) {
        self.close_requested.store(false, Ordering::Relaxed);
        self.opens.fetch_add(1, Ordering::Relaxed);
        // Release pairs with the Acquire in `is_shown`: a frame that sees the library open also sees the new count
        self.open.store(true, Ordering::Release);
    }

    // Game thread: the library opens again where it was, without counting as a new open, so the frame keeps its
    // UI state.
    pub fn reopen(&self) {
        self.close_requested.store(false, Ordering::Relaxed);
        self.open.store(true, Ordering::Release);
    }

    // Game thread: the library shows again after a hide, without counting as a new open.
    pub fn show(&self) {
        self.open.store(true, Ordering::Release);
    }

    // Game thread: the library is hidden and a waiting close request is answered.
    pub fn close(&self) {
        self.open.store(false, Ordering::Release);
        self.close_requested.store(false, Ordering::Relaxed);
    }

    // True while the library is open and no close waits for the tick.
    pub fn is_shown(&self) -> bool {
        self.open.load(Ordering::Acquire) && !self.close_requested.load(Ordering::Relaxed)
    }

    // The number of opens so far. Read after `is_shown`.
    pub fn opens(&self) -> u64 {
        self.opens.load(Ordering::Relaxed)
    }

    // Render thread: the player closed the library. It stops drawing now; the leases go back on the next tick.
    pub fn request_close(&self) {
        self.close_requested.store(true, Ordering::Relaxed);
    }

    // Game thread: true once for each close the frame asked for.
    pub fn take_close_request(&self) -> bool {
        self.close_requested.swap(false, Ordering::Relaxed)
    }

    // Render thread: queues the frame's actions and empties `actions`. Returns how many were dropped because
    // the queue was full; a frame that drops any logs one line.
    pub fn send(&self, actions: &mut Vec<LibraryAction>) -> usize {
        let mut dropped = 0usize;
        for action in actions.drain(..) {
            if self.actions.try_send(action).is_err() {
                dropped += 1;
            }
        }
        if dropped > 0 {
            curia::warn!("library actions dropped, the queue is full", { dropped: dropped });
        }
        dropped
    }

    // Game thread: the oldest queued action.
    pub fn next_action(&self) -> Option<LibraryAction> {
        self.inbox.try_recv().ok()
    }

    // Game thread: makes `snapshot` the one the frame draws.
    pub fn publish(&self, snapshot: Arc<LibrarySnapshot>) {
        let mut current = match self.snapshot.lock() {
            Ok(current) => current,
            Err(poisoned) => poisoned.into_inner(),
        };
        if !Arc::ptr_eq(&current, &snapshot) {
            *current = snapshot;
        }
    }

    // Game thread: the HUD card state the frame draws, or None while the Hud lease is not held.
    pub fn publish_hud(&self, hud: Option<HudState>) {
        let mut current = match self.hud.lock() {
            Ok(current) => current,
            Err(poisoned) => poisoned.into_inner(),
        };
        *current = hud;
    }

    // Render thread: the HUD card state last published, or None while the tick holds the lock, so a frame keeps
    // the state it drew last rather than wait or blink.
    pub fn try_hud(&self) -> Option<Option<HudState>> {
        self.hud.try_lock().ok().map(|current| *current)
    }

    // The snapshot last published. Waits for the lock; the frame uses `try_snapshot`.
    pub fn snapshot(&self) -> Arc<LibrarySnapshot> {
        match self.snapshot.lock() {
            Ok(current) => Arc::clone(&current),
            Err(poisoned) => Arc::clone(&poisoned.into_inner()),
        }
    }

    // Render thread: the snapshot last published, or None while the tick holds the lock.
    pub fn try_snapshot(&self) -> Option<Arc<LibrarySnapshot>> {
        self.snapshot.try_lock().ok().map(|current| Arc::clone(&current))
    }
}
