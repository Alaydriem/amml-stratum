use amml_i18n::localize::Localizer;
use stratum_core::structs::material::StackCount;
use stratum_ui::presenter::{DigitGroups, MaterialRows, StackText};

use crate::support::SnapshotFixture;

#[test]
fn rows_follow_list_order() {
    let names: Vec<String> =
        MaterialRows::build(&SnapshotFixture::list(), "").into_iter().map(|row| row.name).collect();
    assert_eq!(names, ["Stone Bricks", "Spruce Planks", "Deepslate Tiles", "Lantern"]);
}

#[test]
fn filter_is_case_insensitive_substring() {
    let names: Vec<String> =
        MaterialRows::build(&SnapshotFixture::list(), "SPRUCE").into_iter().map(|row| row.name).collect();
    assert_eq!(names, ["Spruce Planks"]);
    let names: Vec<String> =
        MaterialRows::build(&SnapshotFixture::list(), "ric").into_iter().map(|row| row.name).collect();
    assert_eq!(names, ["Stone Bricks"]);
    assert!(MaterialRows::build(&SnapshotFixture::list(), "obsidian").is_empty());
}

#[test]
fn rows_carry_count_and_stacks() {
    let rows = MaterialRows::build(&SnapshotFixture::list(), "");
    assert_eq!(rows[0].count, 3412);
    assert_eq!(rows[0].stacks, StackCount { stacks: 53, rest: 20 });
    assert_eq!(rows[0].stack_size, 64);
}

#[test]
fn stack_text_variants() {
    let english = Localizer::english();
    let text = |stacks: u64, rest: u64| StackText::of(StackCount { stacks, rest }).resolve(&english).into_owned();
    assert_eq!(text(53, 20), "53 stacks + 20");
    assert_eq!(text(24, 0), "24 stacks");
    assert_eq!(text(0, 42), "42");
    assert_eq!(text(1, 0), "1 stack");
    assert_eq!(text(1, 56), "1 stack + 56");
}

#[test]
fn digits_are_grouped_by_three() {
    let grouped: Vec<String> =
        [0, 7, 999, 1000, 4212, 11_904, 1_000_000, 1_234_567, u64::MAX].into_iter().map(DigitGroups::of).collect();
    assert_eq!(
        grouped,
        ["0", "7", "999", "1,000", "4,212", "11,904", "1,000,000", "1,234,567", "18,446,744,073,709,551,615"]
    );
}
