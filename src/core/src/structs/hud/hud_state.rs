use super::{HudControl, HudMode};

// Where the HUD keys stand. Not saved: every session starts in Quick mode on Layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudState {
    pub mode: HudMode,
    pub control: HudControl,
}

impl HudState {
    pub fn new() -> Self {
        Self { mode: HudMode::Quick, control: HudControl::Layer }
    }
}

impl Default for HudState {
    fn default() -> Self {
        Self::new()
    }
}
