use super::{MarkerId, MarkerTarget, MarkerView, Rotation};

// A saved anchor point in one world.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Marker {
    pub id: MarkerId,
    pub name: String,
    pub origin: [i32; 3],
    pub rotation: Rotation,
    pub target: MarkerTarget,
    pub view: MarkerView,
}
