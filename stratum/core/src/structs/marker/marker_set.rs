use super::{Marker, MarkerId, WorldKey};

// Saved markers of one world and dimension. Logic lives in `MarkerService`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MarkerSet {
    pub world: WorldKey,
    #[serde(default)]
    pub markers: Vec<Marker>,
    #[serde(default)]
    pub active: Option<MarkerId>,
    #[serde(default)]
    pub(crate) next_id: u64,
}

impl MarkerSet {
    pub fn new(world: WorldKey) -> Self {
        Self { world, markers: Vec::new(), active: None, next_id: 1 }
    }

    // Pulls every view into its allowed range. A layer past the top is kept: it is read as the
    // whole build when drawn, so a larger structure assigned later shows it again.
    pub(crate) fn clamp_views(&mut self) {
        for marker in &mut self.markers {
            marker.view.opacity_step = marker.view.opacity().index();
        }
    }

    // Raises the id counter past every id in use, so a file with a missing counter cannot reuse an
    // id. An active id that no marker has is dropped.
    pub(crate) fn clamp_counters(&mut self) {
        let next = self.markers.iter().map(|marker| marker.id.0).max().map_or(1, |max| max.saturating_add(1));
        self.next_id = self.next_id.max(next);
        if self.active.is_some_and(|id| !self.markers.iter().any(|marker| marker.id == id)) {
            self.active = None;
        }
    }
}
