use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, TryLockError};

use stratum_core::structs::ghost::{GhostFrame, GhostReport};

use super::{GhostOutbox, GhostTake, RenderMeter, ReportQueue};

// The ghost state the game thread and the render thread share. The tick publishes each tick's frame into the
// outbox; the render thread takes it with try_lock, so it never waits for the tick, and hands its reports back
// through a bounded queue. Each lock is held only to copy or swap one frame's buffers. Each take also times the
// rendered frame for the telemetry.
//
// Bound: the outbox and the take hold one frame each (see their types); at most `ReportQueue::CAPACITY` reports
// wait for a tick, and a report past that is folded into the newest one and counted. Entries a fold cuts count as
// dropped.
pub struct SharedGhost {
    outbox: Mutex<GhostOutbox>,
    take: Mutex<GhostTake>,
    reports: Mutex<ReportQueue>,
    render: RenderMeter,
    // The slot capacity of the last publish, which bounds a report's entries.
    slot_capacity: AtomicU32,
    dropped_reports: AtomicU64,
    dropped_entries: AtomicU64,
    merged_reports: AtomicU64,
    // A report said the renderer reset its world. Sticky, so the reset reaches core even when its report is dropped.
    world_reset: AtomicBool,
}

impl SharedGhost {
    pub fn new() -> Self {
        Self {
            outbox: Mutex::new(GhostOutbox::new()),
            take: Mutex::new(GhostTake::new()),
            reports: Mutex::new(ReportQueue::new()),
            render: RenderMeter::new(),
            slot_capacity: AtomicU32::new(0),
            dropped_reports: AtomicU64::new(0),
            dropped_entries: AtomicU64::new(0),
            merged_reports: AtomicU64::new(0),
            world_reset: AtomicBool::new(false),
        }
    }

    pub fn new_shared() -> Arc<Self> {
        Arc::new(Self::new())
    }

    // The rendered frame times the takes record.
    pub fn render_meter(&self) -> &RenderMeter {
        &self.render
    }

    // Game thread: folds the tick's frame into what the render thread takes next.
    pub fn publish(&self, frame: &GhostFrame) {
        let mut outbox = match self.outbox.lock() {
            Ok(outbox) => outbox,
            Err(poisoned) => poisoned.into_inner(),
        };
        outbox.publish(frame);
        self.slot_capacity.store(frame.slot_capacity, Ordering::Relaxed);
    }

    // Render thread: the frame to apply now. When nothing was published since the last take, or the tick holds
    // the outbox, the take repeats the last draws with no clear, frees, jobs or palette. None after a panic
    // poisoned the take, or while another take is in progress.
    pub fn take(&self) -> Option<MutexGuard<'_, GhostTake>> {
        let mut take = self.take.try_lock().ok()?;
        self.render.frame();
        match self.outbox.try_lock() {
            Ok(mut outbox) if outbox.is_pending() => take.receive(&mut outbox),
            _ => take.repeat(),
        }
        Some(take)
    }

    // The entries of a report with `count` built entries that are read: at most the slot capacity, since a frame
    // hands out at most one job per slot.
    pub fn built_limit(&self, count: u32) -> usize {
        count.min(self.slot_capacity.load(Ordering::Relaxed)) as usize
    }

    // Render thread: queues the report `fill` writes. The report `fill` gets is reused, so `fill` writes every field.
    // Its built entries are cut to the slot capacity, since a frame
    // hands out at most one job per slot. A full queue folds it into its newest report, so no build answer is lost
    // unless its entries pass the limit, and those entries are counted as dropped. A queue the tick
    // holds right now drops the report being sent, since the render thread does not wait. Both are counted. A `world_reset` is kept apart from the report and reaches the tick even when the
    // report is dropped. None after a panic poisoned the queue.
    pub fn report(&self, world_reset: bool, fill: impl FnOnce(&mut GhostReport)) -> Option<()> {
        if world_reset {
            // Release pairs with the Acquire swap in `drain`: a tick that takes the reset also sees every report
            // this thread queued before it
            self.world_reset.store(true, Ordering::Release);
        }
        let limit = self.slot_capacity.load(Ordering::Relaxed) as usize;
        match self.reports.try_lock() {
            Ok(mut queue) => {
                let merged = queue.push(limit, |report| {
                    fill(report);
                    report.built.truncate(limit);
                    report.world_reset = false;
                });
                if let Some(cut) = merged {
                    self.merged_reports.fetch_add(1, Ordering::Relaxed);
                    self.dropped_entries.fetch_add(cut as u64, Ordering::Relaxed);
                }
                Some(())
            }
            Err(TryLockError::WouldBlock) => {
                self.dropped_reports.fetch_add(1, Ordering::Relaxed);
                Some(())
            }
            Err(TryLockError::Poisoned(_)) => None,
        }
    }

    // Game thread: hands each waiting report to `apply`, oldest first, then a waiting world reset once, as one
    // report with no entries. Reports the render thread is queueing right now wait for the next tick.
    pub fn drain(&self, mut apply: impl FnMut(&GhostReport)) {
        if let Ok(mut queue) = self.reports.try_lock() {
            queue.drain(&mut apply);
        }
        // Acquire pairs with the Release store in `report`
        if self.world_reset.swap(false, Ordering::Acquire) {
            let mut reset = GhostReport::new();
            reset.world_reset = true;
            apply(&reset);
        }
    }

    // Session total: whole reports dropped because the tick held the queue.
    pub fn dropped_reports(&self) -> u64 {
        self.dropped_reports.load(Ordering::Relaxed)
    }

    // Session total: build entries cut when a fold passed its limit.
    pub fn dropped_entries(&self) -> u64 {
        self.dropped_entries.load(Ordering::Relaxed)
    }

    // Session total: reports folded into the newest queued report because the queue was full.
    pub fn merged_reports(&self) -> u64 {
        self.merged_reports.load(Ordering::Relaxed)
    }
}

impl Default for SharedGhost {
    fn default() -> Self {
        Self::new()
    }
}
