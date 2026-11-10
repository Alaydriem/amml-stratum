/// Layout mirror of the game's `mce::Mesh` on 26.51 (632 bytes, pointer aligned). Opaque: the mod only moves,
/// renders and destroys meshes through the game's functions and never reads a member, so it needs the size and
/// nothing else. It is the larger of the two mirrors, so a buffer of it holds a mesh of either build.
///
/// The `Mesh.size` entry selects the mirror: this one is for a build whose mesh is [`MeshLayout2651::SIZE`] bytes.
#[repr(C)]
pub struct MeshLayout2651 {
    _bytes: [u64; 79],
}

impl MeshLayout2651 {
    /// The size of the mesh, the value the `Mesh.size` entry has for this layout.
    pub const SIZE: u64 = size_of::<MeshLayout2651>() as u64;
}

const _: () = assert!(size_of::<MeshLayout2651>() == 632);
const _: () = assert!(align_of::<MeshLayout2651>() == 8);
