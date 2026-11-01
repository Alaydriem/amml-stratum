use stratum_core::structs::ghost::SectionPart;

use crate::support::SectionAssert;

#[test]
fn distance_is_measured_to_the_centre_of_the_part() {
    let feet = [8, 8, 8];

    assert_eq!(SectionAssert::whole(0, 0, 0).distance_sq(feet), 0);
    assert_eq!(SectionAssert::key(0, 0, 0, None, SectionPart::Lower).distance_sq(feet), 64);
    assert_eq!(SectionAssert::key(0, 0, 0, None, SectionPart::Upper).distance_sq(feet), 64);
}

#[test]
fn distance_is_equal_for_mirrored_sections_and_uses_the_layer_row() {
    let feet = [8, 8, 8];

    assert_eq!(SectionAssert::whole(1, 0, 0).distance_sq(feet), SectionAssert::whole(-1, 0, 0).distance_sq(feet));
    assert_eq!(SectionAssert::key(0, 0, 0, Some(8), SectionPart::Whole).distance_sq(feet), 1);
}
