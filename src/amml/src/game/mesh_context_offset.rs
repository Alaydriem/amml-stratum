/// Where the `mce::MeshContext` base sits in the game's `ScreenContext`: after the 12-byte `UIScreenContext`
/// base, aligned to 8. `Mesh::renderMesh` takes the `MeshContext&`, so a draw passes the screen address plus this.
pub struct MeshContextOffset;

impl MeshContextOffset {
    pub const BYTES: usize = 16;

    /// The `MeshContext` address inside the screen context at `screen`.
    pub fn from_screen(screen: usize) -> usize {
        screen + Self::BYTES
    }
}
