use alaydriem_stratum::game::MeshLayout2651;

#[test]
fn mesh_layout_2651_is_the_632_byte_mesh_with_pointer_alignment() {
    assert_eq!(size_of::<MeshLayout2651>(), 632);
    assert_eq!(align_of::<MeshLayout2651>(), 8);
}

#[test]
fn an_optional_mesh_slot_fits_the_mirror_and_its_flag() {
    assert_eq!(size_of::<amml_msvc::Optional<MeshLayout2651>>(), 640);
}
