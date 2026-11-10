use alaydriem_stratum::game::{MeshLayout2651, MeshLayout2660};

#[test]
fn mesh_layout_2660_is_the_608_byte_mesh_with_pointer_alignment() {
    assert_eq!(size_of::<MeshLayout2660>(), 608);
    assert_eq!(align_of::<MeshLayout2660>(), 8);
}

#[test]
fn the_2651_mirror_is_large_enough_for_a_mesh_of_either_build() {
    assert!(size_of::<MeshLayout2651>() >= size_of::<MeshLayout2660>());
}
