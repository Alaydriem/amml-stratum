use std::collections::VecDeque;

use stratum_core::structs::ghost::GhostReport;

// The ghost renderer's reports waiting for the tick. Bound: `CAPACITY` reports; a push to a full queue is folded into the
// newest report, so no build answer is lost, and the newest report holds at most the push's `built_limit` entries.
// Reports are reused, so a push allocates only while a report's buffer grows.
pub struct ReportQueue {
    queued: VecDeque<GhostReport>,
    spare: Vec<GhostReport>,
    // Where a push to a full queue is written before it is folded in.
    scratch: GhostReport,
}

impl ReportQueue {
    pub const CAPACITY: usize = 8;

    pub fn new() -> Self {
        Self {
            queued: VecDeque::with_capacity(Self::CAPACITY),
            spare: Vec::with_capacity(Self::CAPACITY),
            scratch: GhostReport::new(),
        }
    }

    // Queues the report `fill` writes. When the queue is full it is folded into the newest queued report, its
    // built entries kept up to `built_limit` in all; the count of entries that did not fit then, else None.
    pub fn push(&mut self, built_limit: usize, fill: impl FnOnce(&mut GhostReport)) -> Option<usize> {
        if self.queued.len() < Self::CAPACITY {
            let mut report = self.spare.pop().unwrap_or_default();
            fill(&mut report);
            self.queued.push_back(report);
            return None;
        }
        fill(&mut self.scratch);
        let cut = match self.queued.back_mut() {
            Some(newest) => newest.absorb(&self.scratch, built_limit),
            None => 0,
        };
        Some(cut)
    }

    // Hands each queued report to `apply`, oldest first, and empties the queue.
    pub fn drain(&mut self, mut apply: impl FnMut(&GhostReport)) {
        while let Some(report) = self.queued.pop_front() {
            apply(&report);
            self.spare.push(report);
        }
    }

    pub fn len(&self) -> usize {
        self.queued.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queued.is_empty()
    }
}

impl Default for ReportQueue {
    fn default() -> Self {
        Self::new()
    }
}
