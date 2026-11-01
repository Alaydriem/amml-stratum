use std::mem::offset_of;

use super::MeshDataLayout;

/// Layout mirror of the start of the game's `Tessellator`, up to `mTessellating`: the mesh data that a build
/// fills, the vertex count that `end` reads, and the flag that `begin` and `end` pair on. A prefix: the mod only
/// reads and writes through the game's pointer. `Tessellator.mTessellating` is the anchor entry.
#[repr(C)]
pub struct TessellatorLayout {
    _head: [u8; 8],                // 0x000
    pub mesh_data: MeshDataLayout, // 0x008 mMeshData
    _between: [u8; 0x298 - 0x168], // 0x168
    pub count: u32,                // 0x298 mCount
    _pad: [u8; 4],                 // 0x29C
    pub tessellating: bool,        // 0x2A0 mTessellating
}

const _: () = assert!(offset_of!(TessellatorLayout, mesh_data) == 8);
const _: () = assert!(offset_of!(TessellatorLayout, count) == 664);
const _: () = assert!(offset_of!(TessellatorLayout, tessellating) == 672);
