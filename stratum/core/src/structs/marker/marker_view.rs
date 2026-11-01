use super::{OpacityStep, ViewMode};

// View settings that belong to one marker.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MarkerView {
    pub shown: bool,
    pub opacity_step: u8,
    pub layer: u32,
    pub view_mode: ViewMode,
    pub easy_place: bool,
    pub selected: u32,
}

impl MarkerView {
    pub fn opacity(&self) -> OpacityStep {
        OpacityStep::new(self.opacity_step)
    }

    // The layer drawn out of `layers`, or None when the whole build is drawn. A layer past the top
    // of a smaller structure counts as the whole build.
    pub fn shown_layer(&self, layers: u32) -> Option<u32> {
        match self.view_mode {
            ViewMode::Layer if self.layer < layers => Some(self.layer),
            _ => None,
        }
    }
}

impl Default for MarkerView {
    fn default() -> Self {
        Self { shown: true, opacity_step: 2, layer: 0, view_mode: ViewMode::Whole, easy_place: false, selected: 0 }
    }
}
