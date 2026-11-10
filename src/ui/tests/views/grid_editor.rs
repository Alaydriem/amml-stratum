use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::layout::{BigBuildLayout, GridCell, LayoutIssue, PiecePlacement};
use stratum_core::structs::library::{ProjectId, ProjectKind, StructureId};
use stratum_core::structs::snapshot::LibrarySnapshot;
use stratum_core::structs::structure::StructureSize;
use stratum_ui::state::Page;

use crate::support::{ShapeProbe, SnapshotFixture, ViewSession};

// A big build whose saved positions overlap: "Harbour west" sits in cell 1, "Harbour east" is not placed.
struct Scenes;

impl Scenes {
    fn snapshot() -> LibrarySnapshot {
        let mut west = SnapshotFixture::structure(1, "Harbour west", Some(7));
        west.grid_cell = Some(GridCell::new(0, 0));
        let east = SnapshotFixture::structure(2, "Harbour east", Some(7));
        let mut project = SnapshotFixture::project(7, "Harbour", vec![west, east]);
        project.kind = ProjectKind::BigBuild;
        let mut snapshot = LibrarySnapshot::empty();
        snapshot.revision = 1;
        snapshot.projects = vec![project];
        snapshot.all_count = 2;
        snapshot.in_world = true;
        snapshot.layouts = vec![(ProjectId(7), Err(LayoutIssue::Overlap(StructureId(1), StructureId(2))))];
        snapshot
    }

    fn session() -> ViewSession {
        let mut session = ViewSession::new(Self::snapshot());
        session.state.page = Page::Project(ProjectId(7));
        session.frame(Vec::new());
        session
    }
}

#[test]
fn grid_editor_offers_only_free_cells() {
    let mut session = Scenes::session();
    assert!(ShapeProbe::text_rect(&session.shapes, "Cell 2").is_some(), "the grid is painted");
    let chip = ShapeProbe::last_text_centre(&session.shapes, "Harbour east").expect("the tray chip is painted");
    let taken = ShapeProbe::last_text_centre(&session.shapes, "Harbour west").expect("the filled cell is painted");
    session.drag(chip, taken);
    assert!(session.actions.is_empty(), "a filled cell takes no drop: {:?}", session.actions);
}

#[test]
fn grid_editor_pushes_set_grid_cell() {
    let mut session = Scenes::session();
    let chip = ShapeProbe::last_text_centre(&session.shapes, "Harbour east").expect("the tray chip is painted");
    let free = ShapeProbe::last_text_centre(&session.shapes, "Cell 2").expect("a free cell is painted");
    session.drag(chip, free);
    assert_eq!(session.actions, [LibraryAction::SetGridCell(StructureId(2), Some(GridCell::new(1, 0)))]);
}

#[test]
fn grid_editor_unplace_pushes_none() {
    let mut session = Scenes::session();
    let placed = ShapeProbe::last_text_centre(&session.shapes, "Harbour west").expect("the filled cell is painted");
    // dropping a placed piece outside the placer returns it to the tray
    let outside = egui::pos2(placed.x, 1070.0);
    session.drag(placed, outside);
    assert_eq!(session.actions, [LibraryAction::SetGridCell(StructureId(1), None)]);
}

#[test]
fn a_completed_grid_stays_editable() {
    let mut west = SnapshotFixture::structure(1, "Harbour west", Some(7));
    west.grid_cell = Some(GridCell::new(0, 0));
    let mut east = SnapshotFixture::structure(2, "Harbour east", Some(7));
    east.grid_cell = Some(GridCell::new(1, 0));
    let mut project = SnapshotFixture::project(7, "Harbour", vec![west, east]);
    project.kind = ProjectKind::BigBuild;
    let mut snapshot = LibrarySnapshot::empty();
    snapshot.revision = 1;
    snapshot.projects = vec![project];
    snapshot.all_count = 2;
    let layout = BigBuildLayout {
        pieces: vec![
            PiecePlacement { structure: StructureId(1), offset: [0, 0, 0] },
            PiecePlacement { structure: StructureId(2), offset: [32, 0, 0] },
        ],
        size: StructureSize::new(64, 18, 24),
    };
    snapshot.layouts = vec![(ProjectId(7), Ok(layout))];
    let mut session = ViewSession::new(snapshot);
    session.state.page = Page::Project(ProjectId(7));
    session.frame(Vec::new());

    assert!(ShapeProbe::text_rect(&session.shapes, "From the grid").is_some(), "the layout facts are painted");
    assert!(ShapeProbe::text_rect(&session.shapes, "GRID").is_some(), "the grid is painted");
    let placed = ShapeProbe::last_text_centre(&session.shapes, "Harbour west").expect("the filled cell is painted");
    let outside = egui::pos2(placed.x, 1070.0);
    session.drag(placed, outside);
    assert_eq!(session.actions, [LibraryAction::SetGridCell(StructureId(1), None)]);
}
