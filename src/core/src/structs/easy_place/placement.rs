use crate::structs::block::Face;

// Place against `neighbour` on its `face`, the face that points at `spot`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placement {
    pub spot: [i32; 3],
    pub neighbour: [i32; 3],
    pub face: Face,
    // The click point inside the neighbour block, from its minimum corner, each axis 0.0 to 1.0, on `face`.
    pub click: [f32; 3],
}
