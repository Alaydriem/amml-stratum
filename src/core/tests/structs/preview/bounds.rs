use stratum_core::structs::preview::PreviewBounds;
use stratum_core::structs::structure::StructureSize;

#[test]
fn an_empty_box_holds_nothing_until_a_corner_is_added() {
    let mut bounds = PreviewBounds::empty();
    assert!(bounds.is_empty());
    bounds.include([16, -32, 48]);
    assert!(!bounds.is_empty());
    assert_eq!((bounds.low(), bounds.high()), ([1.0, -2.0, 3.0], [1.0, -2.0, 3.0]));
}

#[test]
fn corners_grow_the_box_on_each_axis_alone() {
    let mut bounds = PreviewBounds::empty();
    bounds.include([16, 0, -16]);
    bounds.include([-8, 32, 0]);
    assert_eq!(bounds.low(), [-0.5, 0.0, -1.0]);
    assert_eq!(bounds.high(), [1.0, 2.0, 0.0]);
}

#[test]
fn the_structure_box_runs_from_the_origin_and_stops_at_the_largest_sixteenth() {
    let bounds = PreviewBounds::of_size(StructureSize::new(3, 384, 5000));
    assert_eq!(bounds.low(), [0.0, 0.0, 0.0]);
    assert_eq!(bounds.high()[0], 3.0);
    assert_eq!(bounds.high()[1], 384.0);
    // 5000 blocks are past what an i16 of sixteenths holds
    assert_eq!(bounds.high()[2], f32::from(i16::MAX) / 16.0);
}
