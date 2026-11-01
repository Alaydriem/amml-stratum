use alaydriem_stratum::game::MeshContextOffset;

#[test]
fn mesh_context_starts_16_bytes_into_the_screen_context() {
    assert_eq!(MeshContextOffset::BYTES, 16);
    assert_eq!(MeshContextOffset::from_screen(0x7000), 0x7010);
}
