use crate::support::FakeBuffer;
use alaydriem_stratum::game::TessellatorLayout;

#[test]
fn tessellator_layout_reads_mesh_data_count_and_tessellating() {
    let mut buffer = FakeBuffer::new(0x2A8);
    // MeshData::mMode at Tessellator+8, then the positions vector
    buffer.put_u8(0x08, 3);
    buffer.put_u64(0x10, 0x4000);
    buffer.put_u64(0x18, 0x4000 + 24);
    buffer.put_u32(0x298, 77);
    buffer.put_u8(0x2A0, 1);

    let tessellator = buffer.view::<TessellatorLayout>();

    assert_eq!(tessellator.mesh_data.mode, 3);
    assert_eq!(tessellator.mesh_data.positions.len(), 2);
    assert_eq!(tessellator.count, 77);
    assert!(tessellator.tessellating);
}
