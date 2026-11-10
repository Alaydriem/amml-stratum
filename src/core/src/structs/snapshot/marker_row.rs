use crate::structs::library::StructureId;
use crate::structs::marker::{MarkerId, MarkerTarget, TargetState};

// One marker of the loaded world as the UI lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkerRow {
    pub id: MarkerId,
    pub name: String,
    pub target: MarkerTarget,
    // The name of the structure or project the marker shows; empty when it is missing.
    pub target_name: String,
    // For a marker that cycles through a normal project: the name of its current structure and its
    // 1-based place with the structure count. Both are `None` for any other marker.
    pub part_name: Option<String>,
    pub part_position: Option<(u32, u32)>,
    // For a marker that shows a structure of a normal project: that structure. `None` for any other marker.
    pub current: Option<StructureId>,
    pub state: TargetState,
    // The block the marker stands on, x y z, as stored.
    pub origin: [i32; 3],
    pub active: bool,
}
