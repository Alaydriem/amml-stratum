use amml_ui::tokens::{ColorToken, color};
use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::library::{ProjectId, StructureId};
use stratum_core::structs::marker::{MarkerId, MarkerTarget, TargetState};
use stratum_core::structs::snapshot::LibrarySnapshot;
use stratum_ui::presenter::SidebarRow;
use stratum_ui::state::{Dialog, DialogItem, Page, ProjectTab, StructureTab};
use stratum_ui::views::SidebarView;
use stratum_ui::views::dialogs::DeleteDialog;

use crate::support::{RowSpots, ShapeProbe, SnapshotFixture, ViewSession};

// Clicks the sidebar's import button, the left-most one, with `page` open.
struct SidebarImport;

impl SidebarImport {
    fn click_on(page: Page) -> Vec<LibraryAction> {
        let mut session = ViewSession::new(SnapshotFixture::library());
        session.state.page = page;
        session.frame(Vec::new());
        let rects = ShapeProbe::exact_rects(&session.shapes, "Import .mcstructure");
        let sidebar = rects.iter().min_by(|a, b| a.min.x.total_cmp(&b.min.x)).expect("sidebar button").center();
        session.click(sidebar);
        session.actions
    }
}

#[test]
fn import_from_the_sidebar_on_a_project_page_targets_that_project() {
    let actions = SidebarImport::click_on(Page::Project(ProjectId(1)));
    assert_eq!(actions, [LibraryAction::RequestFilePicker(Some(ProjectId(1)))]);
}

#[test]
fn import_from_the_sidebar_on_unsorted_targets_none() {
    let actions = SidebarImport::click_on(Page::Unsorted);
    assert_eq!(actions, [LibraryAction::RequestFilePicker(None)]);
}

#[test]
fn rows_are_rebuilt_only_when_the_revision_changes() {
    let mut view = SidebarView::new();
    let mut snapshot = SnapshotFixture::library();
    snapshot.revision = 5;
    let first = view.rows(&snapshot).len();

    // same revision: the cached rows are returned even though the content differs
    snapshot.unsorted.clear();
    snapshot.markers.clear();
    assert_eq!(view.rows(&snapshot).len(), first);

    snapshot.revision = 6;
    let rows = view.rows(&snapshot);
    assert!(rows.len() < first);
    assert!(rows.iter().all(|row| !matches!(row, SidebarRow::Marker { .. })));
}

// A library with `count` markers named "Marker N" that all show the structure "Main hall".
struct ManyMarkers;

impl ManyMarkers {
    // East wing (shows `target`) and Lighthouse (missing).
    fn two(target: MarkerTarget) -> LibrarySnapshot {
        let mut snapshot = SnapshotFixture::library();
        snapshot.markers = vec![
            SnapshotFixture::marker(1, "East wing", [12, 64, -340], false, TargetState::Ok),
            SnapshotFixture::marker(2, "Lighthouse", [214, 64, -340], false, TargetState::Missing),
        ];
        snapshot.markers[0].target = target;
        snapshot
    }

    fn snapshot(count: u64) -> LibrarySnapshot {
        let mut snapshot = SnapshotFixture::library();
        snapshot.markers = (1..=count)
            .map(|n| {
                SnapshotFixture::marker(n, &format!("Marker {n}"), [10 * n as i32, 64, -340], false, TargetState::Ok)
            })
            .collect();
        snapshot
    }
}

#[test]
fn a_marker_row_shows_its_position_and_no_distance() {
    let mut snapshot = ManyMarkers::two(MarkerTarget::Project(ProjectId(2)));
    snapshot.markers[0].origin = [112, -12, -340];
    let session = ViewSession::new(snapshot);
    let name = ShapeProbe::text_rect(&session.shapes, "East wing").expect("marker row is painted");
    let position = ShapeProbe::exact_rects(&session.shapes, "112 -12 -340")
        .into_iter()
        .find(|rect| (rect.center().y - name.center().y).abs() < 4.0)
        .expect("the position is on the name line");
    assert!(position.left() > name.right(), "the position follows the name");
    assert!(ShapeProbe::text_rect(&session.shapes, "12 m").is_none(), "no distance is painted");
}

#[test]
fn marker_row_click_opens_the_target_materials() {
    let mut session = ViewSession::new(ManyMarkers::two(MarkerTarget::Project(ProjectId(2))));
    let spot = ShapeProbe::text_centre(&session.shapes, "East wing").expect("marker row is painted");
    session.click(spot);
    assert_eq!(session.state.page, Page::Project(ProjectId(2)));
    assert_eq!(session.state.project_tab, ProjectTab::Materials);

    let mut session = ViewSession::new(ManyMarkers::two(MarkerTarget::Structure(StructureId(3))));
    let spot = ShapeProbe::text_centre(&session.shapes, "East wing").expect("marker row is painted");
    session.click(spot);
    assert_eq!(session.state.page, Page::Structure(StructureId(3)));
    assert_eq!(session.state.structure_tab, StructureTab::Materials);
}

#[test]
fn toggle_click_does_not_open_a_page() {
    let mut session = ViewSession::new(ManyMarkers::two(MarkerTarget::Project(ProjectId(2))));
    let spot = RowSpots::toggle(&session, "East wing");
    session.click(spot);
    assert_eq!(session.actions, [LibraryAction::ActivateMarker(MarkerId(1))]);
    assert_eq!(session.state.page, Page::All);
}

#[test]
fn missing_target_click_does_nothing() {
    let mut session = ViewSession::new(ManyMarkers::two(MarkerTarget::Project(ProjectId(2))));
    let spot = ShapeProbe::text_centre(&session.shapes, "Lighthouse").expect("marker row is painted");
    session.click(spot);
    assert_eq!(session.state.page, Page::All);
    assert!(session.actions.is_empty());
}

#[test]
fn open_target_draws_its_marker_selected() {
    let active = <color::BgActive as ColorToken>::VALUE;
    let row_selected = |session: &ViewSession| {
        let label = ShapeProbe::text_centre(&session.shapes, "East wing").expect("marker row is painted");
        ShapeProbe::fill_rects(&session.shapes, active)
            .iter()
            .any(|rect| rect.min.x < session.panel.min.x + 470.0 && rect.y_range().contains(label.y))
    };
    let mut session = ViewSession::new(ManyMarkers::two(MarkerTarget::Structure(StructureId(3))));
    assert!(!row_selected(&session));
    session.state.page = Page::Structure(StructureId(3));
    session.frame(Vec::new());
    assert!(row_selected(&session));
    session.state.page = Page::Structure(StructureId(4));
    session.frame(Vec::new());
    assert!(!row_selected(&session));
}

#[test]
fn filter_shows_only_above_threshold() {
    let session = ViewSession::new(ManyMarkers::snapshot(8));
    assert!(ShapeProbe::text_rect(&session.shapes, "Filter markers").is_none());
    let session = ViewSession::new(ManyMarkers::snapshot(9));
    assert!(ShapeProbe::text_rect(&session.shapes, "Filter markers").is_some());
}

#[test]
fn filter_with_no_match_shows_the_caption() {
    let mut session = ViewSession::new(ManyMarkers::snapshot(9));
    session.state.marker_filter = "zzz".to_owned();
    session.frame(Vec::new());
    assert!(ShapeProbe::text_rect(&session.shapes, "No markers match").is_some());
    assert!(ShapeProbe::text_rect(&session.shapes, "Marker 1").is_none());
    session.state.marker_filter = "marker 9".to_owned();
    session.frame(Vec::new());
    assert!(ShapeProbe::text_rect(&session.shapes, "No markers match").is_none());
    assert!(ShapeProbe::text_rect(&session.shapes, "Marker 9").is_some());
}

#[test]
fn row_delete_opens_the_delete_dialog() {
    let mut session = ViewSession::new(ManyMarkers::two(MarkerTarget::Project(ProjectId(2))));
    let spot = RowSpots::delete(&session, "East wing");
    session.click(spot);
    assert_eq!(session.state.dialog, Some(Dialog::Delete(DeleteDialog::new(DialogItem::Marker(MarkerId(1))))));
    assert_eq!(session.state.page, Page::All);
    assert!(session.actions.is_empty());
}

#[test]
fn row_delete_is_off_when_markers_are_read_only() {
    let mut snapshot = ManyMarkers::two(MarkerTarget::Project(ProjectId(2)));
    snapshot.markers_read_only = true;
    let mut session = ViewSession::new(snapshot);
    let spot = RowSpots::delete(&session, "East wing");
    session.click(spot);
    assert!(session.state.dialog.is_none());
    assert_eq!(session.state.page, Page::All);
    let hover = RowSpots::delete(&session, "East wing");
    session.frame(vec![egui::Event::PointerMoved(hover)]);
    for _ in 0..10 {
        session.frame(Vec::new());
    }
    assert!(
        ShapeProbe::text_rect(&session.shapes, "Markers are read only this session").is_some(),
        "the disabled reason shows"
    );
}

#[test]
fn a_long_marker_name_is_cut_and_the_position_stays_on_the_row() {
    let mut snapshot = ManyMarkers::two(MarkerTarget::Project(ProjectId(2)));
    let name = "W".repeat(64);
    snapshot.markers[0].name = name.clone();
    snapshot.markers[0].origin = [-30000, -64, 29999];
    let session = ViewSession::new(snapshot);
    let cut = ShapeProbe::text_rect(&session.shapes, "WWWW").expect("the cut name is painted");
    // Paper: Markers, the text column of a marker row is 303 wide; 64 W do not fit in it
    assert!(cut.width() < 303.0, "the name {cut:?} is cut to the row");
    let position = ShapeProbe::exact_rects(&session.shapes, "-30000 -64 29999")
        .into_iter()
        .find(|rect| (rect.center().y - cut.center().y).abs() < 4.0)
        .expect("the position is on the name line");
    assert!(cut.right() < position.left(), "the name ends before the position");
    // Paper: Sidebar, 470 wide, 21 padding at the right
    assert!(position.right() <= session.panel.min.x + 470.0 - 21.0, "the position {position:?} leaves the row");
}
