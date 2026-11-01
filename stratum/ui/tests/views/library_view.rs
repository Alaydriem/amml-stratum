use egui::Pos2;
use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::library::ProjectId;
use stratum_core::structs::library::StructureId;
use stratum_core::structs::marker::{MarkerId, TargetState};
use stratum_core::structs::snapshot::ImportPick;
use stratum_core::structs::snapshot::LibrarySnapshot;
use stratum_ui::state::{Page, ProjectTab};

use crate::support::{RowSpots, ShapeProbe, SnapshotFixture, ViewSession};

// Where a test clicks, measured from the painted panel.
struct Spots;

impl Spots {
    fn toggle(session: &ViewSession, name: &str) -> Pos2 {
        RowSpots::toggle(session, name)
    }

    fn close(session: &ViewSession) -> Pos2 {
        // Paper: Page header, 24 padding at the right; the button is 28 wide
        Pos2::new(session.panel.max.x - 24.0 - 14.0, session.panel.min.y + 42.0)
    }
}

// Two markers that can be switched: East wing is on, Hall test is off.
struct SwitchSnapshot;

impl SwitchSnapshot {
    fn build() -> LibrarySnapshot {
        let mut snapshot = SnapshotFixture::library();
        snapshot.markers = vec![
            SnapshotFixture::marker(1, "East wing", [12, 64, -340], true, TargetState::Ok),
            SnapshotFixture::marker(7, "Hall test", [31, 64, -340], false, TargetState::Ok),
        ];
        snapshot
    }
}

#[test]
fn library_view_renders_headless() {
    let session = ViewSession::new(SnapshotFixture::library());
    assert!(session.panel.width() > 0.0 && session.panel.height() > 0.0, "panel rect {:?}", session.panel);
    assert!(session.actions.is_empty(), "a view pass with no input pushed {:?}", session.actions);
    assert!(!session.state.close_requested);
}

#[test]
fn empty_library_renders_headless() {
    let session = ViewSession::new(LibrarySnapshot::empty());
    assert!(session.panel.width() > 0.0 && session.panel.height() > 0.0);
    assert!(session.actions.is_empty());
}

#[test]
fn read_only_library_renders_headless() {
    let mut snapshot = SnapshotFixture::library();
    snapshot.library_read_only = true;
    snapshot.markers_read_only = true;
    let session = ViewSession::new(snapshot);
    assert!(session.panel.width() > 0.0);
    assert!(session.actions.is_empty());
}

#[test]
fn panel_stays_inside_the_screen() {
    let session = ViewSession::new(SnapshotFixture::library());
    assert!(ViewSession::screen().contains_rect(session.panel), "{:?} outside the screen", session.panel);
}

#[test]
fn panel_has_the_paper_size_when_the_screen_has_room() {
    let session = ViewSession::new(SnapshotFixture::library());
    assert!((session.panel.width() - 1474.0).abs() < 4.0, "{:?}", session.panel);
    assert!((session.panel.height() - 900.0).abs() < 4.0, "{:?}", session.panel);
}

#[test]
fn switching_a_marker_on_pushes_activate() {
    let mut session = ViewSession::new(SwitchSnapshot::build());
    let spot = Spots::toggle(&session, "Hall test");
    session.click(spot);
    assert_eq!(session.actions, [LibraryAction::ActivateMarker(MarkerId(7))]);
}

#[test]
fn switching_the_active_marker_off_pushes_deactivate() {
    let mut session = ViewSession::new(SwitchSnapshot::build());
    let spot = Spots::toggle(&session, "East wing");
    session.click(spot);
    assert_eq!(session.actions, [LibraryAction::DeactivateMarker]);
}

#[test]
fn import_button_pushes_the_file_picker_request() {
    let mut session = ViewSession::new(SnapshotFixture::library());
    let spot = ShapeProbe::text_centre(&session.shapes, "Import .mcstructure").expect("import button is painted");
    session.click(spot);
    assert_eq!(session.actions, [LibraryAction::RequestFilePicker(None)]);
}

#[test]
fn import_button_does_nothing_while_the_picker_is_open() {
    let mut snapshot = SnapshotFixture::library();
    snapshot.picker_pending = true;
    let mut session = ViewSession::new(snapshot);
    let spot = ShapeProbe::text_centre(&session.shapes, "Import .mcstructure").expect("import button is painted");
    session.click(spot);
    assert!(session.actions.is_empty(), "{:?}", session.actions);
}

#[test]
fn import_button_does_nothing_when_the_library_is_read_only() {
    let mut snapshot = SnapshotFixture::library();
    snapshot.library_read_only = true;
    let mut session = ViewSession::new(snapshot);
    let spot = ShapeProbe::text_centre(&session.shapes, "Import .mcstructure").expect("import button is painted");
    session.click(spot);
    assert!(session.actions.is_empty(), "{:?}", session.actions);
}

#[test]
fn close_button_asks_the_host_to_close() {
    let mut session = ViewSession::new(SnapshotFixture::library());
    let spot = Spots::close(&session);
    session.click(spot);
    assert!(session.state.close_requested);
    assert!(session.actions.is_empty());
}

#[test]
fn clicking_a_project_row_opens_its_page() {
    let mut session = ViewSession::new(SnapshotFixture::library());
    assert_eq!(session.state.page, Page::All);
    let spot = ShapeProbe::text_centre(&session.shapes, "Harbour").expect("project row is painted");
    session.click(spot);
    assert_eq!(session.state.page, Page::Project(ProjectId(1)));
    assert!(session.actions.is_empty());
}

#[test]
fn pick_selects_the_structures_tab() {
    let mut session = ViewSession::new(SnapshotFixture::library());
    session.state.page = Page::Project(ProjectId(2));
    session.state.project_tab = ProjectTab::Materials;
    let mut pick = ImportPick::new(1, Some(ProjectId(1)));
    pick.structures = vec![StructureId(3)];
    session.snapshot.latest_pick = Some(pick);
    session.frame(Vec::new());
    assert_eq!(session.state.page, Page::Project(ProjectId(1)));
    assert_eq!(session.state.project_tab, ProjectTab::Structures);
}
