use crate::structs::structure::StructureSize;

// The numbers of one render, for the log and the corner of the pane.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreviewStats {
    pub render_us: u64,
    // Time to read the file and build the mesh; zero when the mesh was kept from an earlier request.
    pub mesh_us: u64,
    pub quads: usize,
    // Triangles that passed the back face test and were rasterised.
    pub triangles: usize,
    pub width: u32,
    pub height: u32,
    pub size: StructureSize,
    // Decoded textures the texture pack holds after this render.
    pub textures_cached: usize,
}
