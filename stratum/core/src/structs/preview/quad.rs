// One textured quad of the preview mesh. Corners are in sixteenths of a block from the structure's origin, and
// texture coordinates in sixteenths of the texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreviewQuad {
    pub corners: [[i16; 3]; 4],
    pub uvs: [[u8; 2]; 4],
    // The index in the mesh's texture table.
    pub texture: u16,
    // The flat light of the face, out of 255.
    pub shade: u8,
    // The outward axis normal; zero for a quad seen from both sides.
    pub normal: [i8; 3],
}
