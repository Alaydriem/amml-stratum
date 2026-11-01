use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::library::StructureId;
use stratum_core::structs::marker::MarkerTarget;
use stratum_core::structs::snapshot::LibrarySnapshot;
use stratum_ui::state::Page;

use crate::support::{RowSpots, ShapeProbe, SnapshotFixture, ViewSession};

#[test]
fn an_unsorted_row_opens_its_structure_page() {
    let mut session = ViewSession::new(SnapshotFixture::library());
    session.state.page = Page::Unsorted;
    session.frame(Vec::new());
    assert!(ShapeProbe::text_rect(&session.shapes, "mill_wheel").is_some(), "the unsorted rows are listed");
    assert!(ShapeProbe::text_rect(&session.shapes, "Harbour west").is_none(), "project rows are not");
    let spot = ShapeProbe::text_centre(&session.shapes, "dock_crane").expect("row is painted");
    session.click(spot);
    assert_eq!(session.state.page, Page::Structure(StructureId(4)));
}

#[test]
fn the_all_page_lists_project_and_unsorted_rows() {
    let mut session = ViewSession::new(SnapshotFixture::library());
    session.state.page = Page::All;
    session.frame(Vec::new());
    for name in ["Harbour west", "mill_wheel", "dock_crane"] {
        assert!(ShapeProbe::text_rect(&session.shapes, name).is_some(), "{name} is listed");
    }
    let spot = ShapeProbe::text_centre(&session.shapes, "Harbour west").expect("row is painted");
    session.click(spot);
    assert_eq!(session.state.page, Page::Structure(StructureId(3)));
}

#[test]
fn the_all_page_sorts_by_name_ignoring_case() {
    let mut session = ViewSession::new(SnapshotFixture::library());
    session.state.page = Page::All;
    session.frame(Vec::new());
    let top = |name: &str| ShapeProbe::text_rect(&session.shapes, name).expect("row is painted").top();
    assert!(top("dock_crane") < top("Harbour west"), "d before h");
    assert!(top("Harbour west") < top("mill_wheel"), "h before m");
}

#[test]
fn an_empty_unsorted_page_says_so() {
    let mut snapshot: LibrarySnapshot = SnapshotFixture::library();
    snapshot.unsorted.clear();
    let mut session = ViewSession::new(snapshot);
    session.state.page = Page::Unsorted;
    session.frame(Vec::new());
    assert!(ShapeProbe::text_rect(&session.shapes, "Nothing is unsorted").is_some());
}

#[test]
fn row_action_places_a_marker_for_that_structure_at_once() {
    let mut session = ViewSession::new(SnapshotFixture::library());
    session.state.page = Page::Unsorted;
    session.frame(Vec::new());
    let spot = RowSpots::structure_action(&mut session, "dock_crane").expect("row is hovered");
    session.click(spot);
    let target = MarkerTarget::Structure(StructureId(4));
    assert_eq!(session.actions, [LibraryAction::PlaceMarker { name: "dock_crane".to_owned(), target }]);
    assert_eq!(session.state.dialog, None, "no dialog opens");
    assert_eq!(session.state.page, Page::Unsorted);
}

#[test]
fn row_place_action_is_off_when_markers_are_read_only() {
    let mut snapshot = SnapshotFixture::library();
    snapshot.markers_read_only = true;
    let mut session = ViewSession::new(snapshot);
    session.state.page = Page::All;
    session.frame(Vec::new());
    let spot = RowSpots::structure_action(&mut session, "dock_crane").expect("row is hovered");
    session.click(spot);
    assert!(session.actions.is_empty(), "{:?}", session.actions);
    assert_eq!(session.state.dialog, None);
    assert_eq!(session.state.page, Page::All);
}

#[test]
fn row_place_action_is_off_in_a_world_without_a_player() {
    let mut snapshot = SnapshotFixture::library();
    snapshot.has_player = false;
    let mut session = ViewSession::new(snapshot);
    session.state.page = Page::All;
    session.frame(Vec::new());
    let spot = RowSpots::structure_action(&mut session, "dock_crane").expect("row is hovered");
    session.click(spot);
    assert_eq!(session.state.dialog, None);
    assert!(session.actions.is_empty(), "{:?}", session.actions);
}

#[test]
fn row_place_action_is_off_outside_a_world() {
    let mut snapshot = SnapshotFixture::library();
    snapshot.in_world = false;
    let mut session = ViewSession::new(snapshot);
    session.state.page = Page::All;
    session.frame(Vec::new());
    let spot = RowSpots::structure_action(&mut session, "dock_crane").expect("row is hovered");
    session.click(spot);
    assert_eq!(session.state.dialog, None);
    assert!(session.actions.is_empty(), "{:?}", session.actions);
    assert_eq!(session.state.page, Page::All);
}
