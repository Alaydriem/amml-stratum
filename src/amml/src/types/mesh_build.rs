use crate::game::OwnedMesh;

/// What one begin, fill and end on the game's tessellator gave.
pub struct MeshBuild {
    /// What the fill step returned.
    pub filled: bool,
    /// The vertices the tessellator held before the end.
    pub vertices: usize,
    /// The mesh the end made; `None` when the end made none.
    pub mesh: Option<OwnedMesh>,
}
