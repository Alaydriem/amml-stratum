use crate::support::FakeBuffer;
use alaydriem_stratum::game::MeshDataLayout;

#[test]
fn mesh_data_layout_reads_the_header_offsets() {
    let mut buffer = FakeBuffer::new(352);
    buffer.put_u8(0, 4);
    // positions: first, last and end of three vec3
    buffer.put_u64(0x08, 0x1000);
    buffer.put_u64(0x10, 0x1000 + 36);
    buffer.put_u64(0x18, 0x1000 + 36);
    buffer.put_u64(0x68, 0x2000);
    buffer.put_u64(0x70, 0x2000 + 8);
    buffer.put_u64(0x110, 0x3000);
    buffer.put_u64(0x118, 0x3000 + 5);
    buffer.put_f32(0x128, -1.5);
    buffer.put_f32(0x13C, 9.25);
    buffer.put_f32(0x140, 0.125);
    buffer.put_f32(0x14C, 0.875);
    buffer.put_u8(0x150, 1);
    buffer.put_u8(0x15E, 1);

    let data = buffer.view::<MeshDataLayout>();

    assert_eq!(data.mode, 4);
    assert_eq!(data.positions.len(), 3);
    assert_eq!(data.colors.len(), 2);
    assert_eq!(data.geo_type.len(), 5);
    assert_eq!(data.aabb[0][0], -1.5);
    assert_eq!(data.aabb[1][2], 9.25);
    assert_eq!(data.uv_aabb[0][0], 0.125);
    assert_eq!(data.uv_aabb[1][1], 0.875);
    assert!(data.field_enabled[0]);
    assert!(data.field_enabled[14]);
    assert!(!data.field_enabled[1]);
}

#[test]
fn mesh_data_layout_is_352_bytes() {
    assert_eq!(size_of::<MeshDataLayout>(), 352);
}
