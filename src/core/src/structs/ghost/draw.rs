// One built slot to draw this frame. The mesh is placed at `origin + scale * (offset + position)`,
// where `position` is a vertex of the slot's mesh.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GhostDraw {
    pub slot: u32,
    // World position of the cell the slot's mesh is placed from.
    pub origin: [i32; 3],
    // Mesh cells from `origin` to the slot's first cell, before the scale. Zero for the full ghost.
    pub offset: [i32; 3],
    // Factor applied to the mesh positions about `origin`: 1.0 for the full ghost and the marker, below 1.0 for the
    // mini ghost.
    pub scale: f32,
}
