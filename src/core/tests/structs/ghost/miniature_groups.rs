use stratum_core::structs::ghost::{MiniatureGroups, SectionKey, SectionPart};

use crate::support::SectionAssert;

#[test]
fn cut_keeps_order_and_the_cell_limit() {
    let keys = [
        SectionAssert::whole(0, 0, 0),
        SectionAssert::whole(1, 0, 0),
        SectionAssert::whole(2, 0, 0),
        SectionAssert::whole(3, 0, 0),
    ];
    let mut groups = MiniatureGroups::new();

    groups.cut(&keys, &[10_000, 6_000, 1_000, 9_000]);

    assert_eq!(groups.group_count(), 2);
    assert_eq!(groups.members(0), &keys[..2]);
    assert_eq!(groups.members(1), &keys[2..]);
}

#[test]
fn a_key_over_the_limit_is_a_group_of_its_own() {
    let keys = [SectionAssert::whole(0, 0, 0), SectionAssert::whole(1, 0, 0), SectionAssert::whole(2, 0, 0)];
    let mut groups = MiniatureGroups::new();

    groups.cut(&keys, &[100, 20_000, 100]);

    assert_eq!(groups.group_count(), 3);
    assert_eq!(groups.members(0), &keys[..1]);
    assert_eq!(groups.members(1), &keys[1..2]);
    assert_eq!(groups.members(2), &keys[2..]);
}

#[test]
fn a_small_build_is_one_group() {
    let keys = [SectionAssert::whole(0, 0, 0), SectionAssert::whole(1, 0, 0), SectionAssert::whole(2, 0, 0)];
    let mut groups = MiniatureGroups::new();

    groups.cut(&keys, &[10, 10, 10]);

    assert_eq!(groups.group_count(), 1);
    assert_eq!(groups.members(0), &keys);
    assert_eq!(
        MiniatureGroups::key(0),
        SectionKey { x: 0, y: 0, z: 0, layer: None, part: SectionPart::MiniatureGroup }
    );
}

#[test]
fn cut_replaces_and_clear_empties() {
    let keys = [SectionAssert::whole(0, 0, 0), SectionAssert::whole(1, 0, 0)];
    let mut groups = MiniatureGroups::new();
    groups.cut(&keys, &[MiniatureGroups::GROUP_CELLS, MiniatureGroups::GROUP_CELLS]);
    assert_eq!(groups.group_count(), 2);

    groups.cut(&keys[..1], &[5]);
    assert_eq!(groups.group_count(), 1);
    assert_eq!(groups.members(0), &keys[..1]);

    groups.clear();
    assert_eq!(groups.group_count(), 0);
    assert!(groups.members(0).is_empty());
}

#[test]
fn keys_lists_one_key_per_group_in_order() {
    let keys = [SectionAssert::whole(0, 0, 0), SectionAssert::whole(1, 0, 0), SectionAssert::whole(2, 0, 0)];
    let mut groups = MiniatureGroups::new();
    groups.cut(&keys, &[MiniatureGroups::GROUP_CELLS; 3]);

    let out: Vec<SectionKey> = groups.keys().collect();

    assert_eq!(out, [MiniatureGroups::key(0), MiniatureGroups::key(1), MiniatureGroups::key(2)]);
}

#[test]
fn keys_totalling_exactly_the_limit_stay_one_group() {
    let keys = [SectionAssert::whole(0, 0, 0), SectionAssert::whole(1, 0, 0), SectionAssert::whole(2, 0, 0)];
    let mut groups = MiniatureGroups::new();

    groups.cut(&keys, &[8_192, 8_192, 1]);

    assert_eq!(groups.group_count(), 2);
    assert_eq!(groups.members(0), &keys[..2]);
    assert_eq!(groups.members(1), &keys[2..]);
}
