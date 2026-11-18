use std::sync::Arc;

use super::{HudControl, HudMode};
use crate::structs::snapshot::BuildRow;

// Where the HUD keys stand. Not saved: every session starts in Quick mode on Layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudState {
    pub mode: HudMode,
    pub control: HudControl,
    // The build card; None when no build is open. Filled by the controller, never by the keys. Shared, so a frame
    // that copies the state does not copy the row.
    pub build: Option<Arc<BuildRow>>,
}

impl HudState {
    pub fn new() -> Self {
        Self { mode: HudMode::Quick, control: HudControl::Layer, build: None }
    }
}

impl Default for HudState {
    fn default() -> Self {
        Self::new()
    }
}
