use std::mem::offset_of;

use amml_msvc::MsvcVec;

/// Layout mirror of the game's `mce::MeshData` (352 bytes): the primitive mode, the 13 vertex and index vectors,
/// the position and UV bounds, and the per-field enable flags.
#[repr(C)]
pub struct MeshDataLayout {
    pub mode: u8,                            // 0x000 mMode
    pub positions: MsvcVec<[f32; 3]>,        // 0x008 mPositions
    pub normals: MsvcVec<[f32; 4]>,          // 0x020 mNormals
    pub tangents: MsvcVec<[f32; 4]>,         // 0x038 mTangents
    pub indices: MsvcVec<u32>,               // 0x050 mIndices
    pub colors: MsvcVec<u32>,                // 0x068 mColors
    pub bone_id0s: MsvcVec<u16>,             // 0x080 mBoneId0s
    pub texture_uvs: [MsvcVec<[f32; 2]>; 3], // 0x098 mTextureUVs
    pub pbr_texture_indices: MsvcVec<u16>,   // 0x0E0 mPBRTextureIndices
    pub mers: MsvcVec<u32>,                  // 0x0F8 mMERS
    pub geo_type: MsvcVec<u8>,               // 0x110 mGeoType
    pub aabb: [[f32; 3]; 2],                 // 0x128 mAABB
    pub uv_aabb: [[f32; 2]; 2],              // 0x140 mUVAABB
    pub field_enabled: [bool; 15],           // 0x150 mFieldEnabled
}

const _: () = assert!(offset_of!(MeshDataLayout, positions) == 0x08);
const _: () = assert!(offset_of!(MeshDataLayout, normals) == 0x20);
const _: () = assert!(offset_of!(MeshDataLayout, tangents) == 0x38);
const _: () = assert!(offset_of!(MeshDataLayout, indices) == 0x50);
const _: () = assert!(offset_of!(MeshDataLayout, colors) == 0x68);
const _: () = assert!(offset_of!(MeshDataLayout, bone_id0s) == 0x80);
const _: () = assert!(offset_of!(MeshDataLayout, texture_uvs) == 0x98);
const _: () = assert!(offset_of!(MeshDataLayout, pbr_texture_indices) == 0xE0);
const _: () = assert!(offset_of!(MeshDataLayout, mers) == 0xF8);
const _: () = assert!(offset_of!(MeshDataLayout, geo_type) == 0x110);
const _: () = assert!(offset_of!(MeshDataLayout, aabb) == 0x128);
const _: () = assert!(offset_of!(MeshDataLayout, uv_aabb) == 0x140);
const _: () = assert!(offset_of!(MeshDataLayout, field_enabled) == 0x150);
const _: () = assert!(size_of::<MeshDataLayout>() == 352);
