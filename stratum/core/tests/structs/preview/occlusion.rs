use stratum_core::structs::preview::Occlusion;

#[test]
fn opaque_hides_everything_and_a_group_hides_only_its_own() {
    assert!(Occlusion::None.hidden_by(Occlusion::Opaque));
    assert!(Occlusion::Group(3).hidden_by(Occlusion::Group(3)));
    assert!(!Occlusion::Group(3).hidden_by(Occlusion::Group(4)));
    assert!(!Occlusion::Opaque.hidden_by(Occlusion::Group(3)));
    assert!(!Occlusion::None.hidden_by(Occlusion::None));
}
