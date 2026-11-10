use stratum_core::structs::library::{ProjectId, StructureId};
use stratum_core::structs::snapshot::ImportPick;
use stratum_ui::state::{Page, PickFlash};

struct Picks;

impl Picks {
    fn of(serial: u64, destination: Option<u64>, ids: &[u64]) -> ImportPick {
        let mut pick = ImportPick::new(serial, destination.map(ProjectId));
        pick.structures = ids.iter().copied().map(StructureId).collect();
        pick
    }
}

#[test]
fn new_pick_moves_to_its_page_once() {
    let mut flash = PickFlash::new();
    let pick = Picks::of(1, Some(7), &[10]);
    assert_eq!(flash.observe(Some(&pick), 0.0), Some(Page::Project(ProjectId(7))));
    assert_eq!(flash.observe(Some(&pick), 0.1), None);
}

#[test]
fn unsorted_pick_moves_to_unsorted() {
    let mut flash = PickFlash::new();
    assert_eq!(flash.observe(Some(&Picks::of(1, None, &[10])), 0.0), Some(Page::Unsorted));
}

#[test]
fn empty_pick_does_not_move_the_page() {
    let mut flash = PickFlash::new();
    assert_eq!(flash.observe(Some(&Picks::of(1, Some(7), &[])), 0.0), None);
    assert_eq!(flash.observe(None, 0.1), None);
}

#[test]
fn late_rows_of_the_same_pick_flash_without_a_page_move() {
    let mut flash = PickFlash::new();
    assert_eq!(flash.observe(Some(&Picks::of(1, Some(7), &[10])), 0.0), Some(Page::Project(ProjectId(7))));
    assert_eq!(flash.observe(Some(&Picks::of(1, Some(7), &[10, 11])), 1.0), None);
    assert_eq!(flash.strength(StructureId(11), 1.0), 1.0);
    assert!((flash.strength(StructureId(10), 1.0) - 0.5).abs() < 1e-6);
}

#[test]
fn rows_arriving_after_an_empty_start_move_the_page_then() {
    let mut flash = PickFlash::new();
    assert_eq!(flash.observe(Some(&Picks::of(1, Some(7), &[])), 0.0), None);
    assert_eq!(flash.observe(Some(&Picks::of(1, Some(7), &[10])), 0.5), Some(Page::Project(ProjectId(7))));
}

#[test]
fn strength_fades_over_flash_seconds() {
    let mut flash = PickFlash::new();
    flash.observe(Some(&Picks::of(1, None, &[10])), 5.0);
    let id = StructureId(10);
    assert_eq!(flash.strength(id, 5.0), 1.0);
    assert!((flash.strength(id, 6.0) - 0.5).abs() < 1e-6);
    assert_eq!(flash.strength(id, 7.0), 0.0);
    assert_eq!(flash.strength(id, 9.0), 0.0);
}

#[test]
fn next_pick_replaces_the_old_rows() {
    let mut flash = PickFlash::new();
    flash.observe(Some(&Picks::of(1, None, &[10, 11, 12])), 0.0);
    assert_eq!(flash.tracked(), 3);
    assert_eq!(flash.observe(Some(&Picks::of(2, Some(3), &[20])), 0.5), Some(Page::Project(ProjectId(3))));
    assert_eq!(flash.strength(StructureId(10), 0.5), 0.0);
    assert_eq!(flash.strength(StructureId(20), 0.5), 1.0);
    assert_eq!(flash.tracked(), 1);
}

#[test]
fn scroll_is_asked_once_for_the_first_row() {
    let mut flash = PickFlash::new();
    flash.observe(Some(&Picks::of(1, None, &[10, 11])), 0.0);
    assert!(!flash.take_scroll(StructureId(11)));
    assert!(flash.take_scroll(StructureId(10)));
    assert!(!flash.take_scroll(StructureId(10)));
}
