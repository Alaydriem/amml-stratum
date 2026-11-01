use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

// The stop of the preview thread, shared by the service that sets it and the mesher and rasteriser that check
// it. Once set it stays set.
#[derive(Debug, Clone)]
pub struct PreviewStop {
    stopped: Arc<AtomicBool>,
}

impl PreviewStop {
    #[expect(clippy::new_without_default, reason = "rule 12: one constructor, `new`")]
    pub fn new() -> Self {
        Self { stopped: Arc::new(AtomicBool::new(false)) }
    }

    pub fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
    }

    pub fn is_stopped(&self) -> bool {
        self.stopped.load(Ordering::SeqCst)
    }
}
