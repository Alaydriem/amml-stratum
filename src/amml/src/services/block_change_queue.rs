use std::sync::atomic::{AtomicU64, Ordering};

use crossbeam_channel::{Receiver, Sender, TrySendError};

use crate::types::QueuedChange;

// The block changes that came while the core was held, so none is lost: the listener pushes without a lock and the
// next tick drains them in order, before the core's own tick.
//
// Bound: at most `CAPACITY` changes wait; a change past that is dropped and counted.
pub struct BlockChangeQueue {
    sender: Sender<QueuedChange>,
    receiver: Receiver<QueuedChange>,
    dropped: AtomicU64,
}

impl BlockChangeQueue {
    pub const CAPACITY: usize = 4096;

    pub fn new() -> Self {
        let (sender, receiver) = crossbeam_channel::bounded(Self::CAPACITY);
        Self { sender, receiver, dropped: AtomicU64::new(0) }
    }

    // false, counting it, when the queue is full
    pub fn push(&self, change: QueuedChange) -> bool {
        match self.sender.try_send(change) {
            Ok(()) => true,
            Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => {
                self.dropped.fetch_add(1, Ordering::Relaxed);
                false
            }
        }
    }

    // hands every waiting change to `apply`, oldest first
    pub fn drain(&self, mut apply: impl FnMut(QueuedChange)) {
        while let Ok(change) = self.receiver.try_recv() {
            apply(change);
        }
    }

    // drops every waiting change
    pub fn clear(&self) {
        self.drain(drop);
    }

    // the changes dropped since the last call
    pub fn take_dropped(&self) -> u64 {
        self.dropped.swap(0, Ordering::Relaxed)
    }
}

impl Default for BlockChangeQueue {
    fn default() -> Self {
        Self::new()
    }
}
