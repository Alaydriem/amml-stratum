use crate::game::MeshLayout2651;

/// Layout mirror of the game's `mce::Mesh` on 26.60 (608 bytes, pointer aligned): 24 bytes smaller than
/// [`MeshLayout2651`], as the base lost its vtable pointer and the members after the MeshData moved (26.60
/// `Mesh::~Mesh`: -8h before the MeshData, -18h at the buffer handles). Opaque, as the 26.51 mirror.
///
/// The `Mesh.size` entry selects the mirror: this one is for a build whose mesh is [`MeshLayout2660::SIZE`] bytes.
#[repr(C)]
pub struct MeshLayout2660 {
    _bytes: [u64; 76],
}

impl MeshLayout2660 {
    /// The size of the mesh, the value the `Mesh.size` entry has for this layout.
    pub const SIZE: u64 = size_of::<MeshLayout2660>() as u64;
}

const _: () = assert!(size_of::<MeshLayout2660>() == 608);
const _: () = assert!(align_of::<MeshLayout2660>() == 8);
const _: () = assert!(size_of::<MeshLayout2651>() >= size_of::<MeshLayout2660>());
