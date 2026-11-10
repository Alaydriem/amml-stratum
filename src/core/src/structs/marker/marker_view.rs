use super::{OpacityStep, PartView, ViewMode};
use crate::structs::library::StructureId;

// View settings that belong to one marker.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MarkerView {
    pub shown: bool,
    pub opacity_step: u8,
    pub layer: u32,
    pub view_mode: ViewMode,
    pub easy_place: bool,
    // The structure of a normal project that is shown, or None before one is chosen.
    #[serde(default)]
    pub current: Option<StructureId>,
    // Saved views of the structures not shown, most recent first. At most `MAX_PARTS`.
    #[serde(default)]
    pub parts: Vec<PartView>,
    // The index key of files from before `current`. It is read and never written.
    #[serde(default, rename = "selected", skip_serializing)]
    pub(crate) legacy_selected: Option<u32>,
}

impl MarkerView {
    // The most saved part views one marker keeps.
    pub const MAX_PARTS: usize = 64;

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

    // Shows `next`. The layer and mode of the structure left are saved, and those saved for `next`
    // come back; a structure not seen before opens at layer 0 and the whole build.
    pub fn switch_to(&mut self, next: StructureId) {
        if self.current == Some(next) {
            return;
        }
        if let Some(previous) = self.current {
            self.parts.retain(|part| part.structure != previous);
            self.parts.insert(0, PartView { structure: previous, layer: self.layer, view_mode: self.view_mode });
            self.parts.truncate(Self::MAX_PARTS);
        }
        self.current = Some(next);
        match self.parts.iter().position(|part| part.structure == next) {
            Some(index) => {
                let part = self.parts.remove(index);
                self.layer = part.layer;
                self.view_mode = part.view_mode;
            }
            None => {
                self.layer = 0;
                self.view_mode = ViewMode::Whole;
            }
        }
    }

    // Makes room for the most parts a marker keeps, so `switch_to` never grows the list.
    pub fn reserve_parts(&mut self) {
        let wanted = Self::MAX_PARTS + 1;
        if self.parts.capacity() < wanted {
            self.parts.reserve_exact(wanted - self.parts.len());
        }
    }

    pub fn forget_parts(&mut self) {
        self.parts.clear();
    }

    pub fn keep_parts(&mut self, keep: &[StructureId]) {
        self.parts.retain(|part| keep.contains(&part.structure));
    }
}

impl Default for MarkerView {
    fn default() -> Self {
        Self {
            shown: true,
            opacity_step: 2,
            layer: 0,
            view_mode: ViewMode::Whole,
            easy_place: false,
            current: None,
            parts: Vec::new(),
            legacy_selected: None,
        }
    }
}
