use crate::structs::marker::{MarkerId, MarkerTarget, TargetState};

// One marker of the loaded world as the UI lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkerRow {
    pub id: MarkerId,
    pub name: String,
    pub target: MarkerTarget,
    // The name of the structure or project the marker shows; empty when it is missing.
    pub target_name: String,
    pub state: TargetState,
    // The block the marker stands on, x y z, as stored.
    pub origin: [i32; 3],
    pub active: bool,
}
