use std::sync::Arc;

use crate::services::{StratumListeners, StratumRuntime};

// what one enable runs: the runtime and the listeners that hand it the events
pub struct StratumRunning {
    pub runtime: Arc<StratumRuntime>,
    pub listeners: StratumListeners,
}
