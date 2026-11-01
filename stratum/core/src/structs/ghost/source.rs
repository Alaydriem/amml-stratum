use crate::structs::marker::{Marker, MarkerId, MarkerTarget, Rotation};

// The parts of a marker that decide which blocks the ghost shows and where. A rename is not one of
// them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BuildSource {
    pub marker: MarkerId,
    pub origin: [i32; 3],
    pub rotation: Rotation,
    pub target: MarkerTarget,
    pub selected: u32,
}

impl BuildSource {
    pub fn new(marker: &Marker) -> Self {
        Self {
            marker: marker.id,
            origin: marker.origin,
            rotation: marker.rotation,
            target: marker.target,
            selected: marker.view.selected,
        }
    }

    // True when both name the same structures, wherever they are placed and however turned.
    pub fn same_blocks(&self, other: &BuildSource) -> bool {
        self.marker == other.marker && self.target == other.target && self.selected == other.selected
    }
}
