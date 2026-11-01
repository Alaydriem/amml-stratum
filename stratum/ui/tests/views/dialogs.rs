use egui::{Key, Pos2};
use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::library::{ProjectId, ProjectKind, StructureId};
use stratum_core::structs::marker::{MarkerId, MarkerTarget};
use stratum_core::structs::snapshot::LibrarySnapshot;
use stratum_ui::state::{Dialog, DialogItem, Page};
use stratum_ui::views::dialogs::{AssignDialog, AssignSubject, RenameDialog};

use crate::support::{ShapeProbe, SnapshotFixture, ViewSession};

// Sessions that start on the page of the structure "Main hall" (id 1), as on the Rename artboard.
struct Scenes;

impl Scenes {
    fn structure_page(snapshot: LibrarySnapshot) -> ViewSession {
        let mut session = ViewSession::new(snapshot);
        session.state.page = Page::Structure(StructureId(1));
        session.frame(Vec::new());
        session
    }

    fn click_first(session: &mut ViewSession, label: &str) {
        let spot = ShapeProbe::text_centre(&session.shapes, label).expect("button is painted");
        session.click(spot);
    }

    fn click_last(session: &mut ViewSession, label: &str) {
        let spot = ShapeProbe::last_text_centre(&session.shapes, label).expect("dialog button is painted");
        session.click(spot);
    }

    // Opens a dropdown by its painted text, then picks the option painted `option`.
    fn choose(session: &mut ViewSession, field: &str, option: &str) {
        Self::click_last(session, field);
        Self::click_last(session, option);
    }

    fn click_exact(session: &mut ViewSession, text: &str) {
        let spot = ShapeProbe::last_exact_centre(&session.shapes, text).expect("text is painted");
        session.click(spot);
    }

    fn open_delete(session: &mut ViewSession) {
        Self::click_first(session, "Delete");
        assert!(session.state.dialog.is_some());
    }

    fn open_dialog(session: &mut ViewSession, dialog: Dialog) {
        session.state.dialog = Some(dialog);
        // a new modal paints from its second frame
        session.frame(Vec::new());
        session.frame(Vec::new());
    }
}

#[test]
fn delete_confirmation_lists_markers() {
    let mut session = Scenes::structure_page(SnapshotFixture::library());
    Scenes::click_first(&mut session, "Delete");
    assert!(session.actions.is_empty(), "asking must not delete: {:?}", session.actions);
    assert!(session.state.dialog.is_some(), "the delete question is open");
    assert!(ShapeProbe::text_rect(&session.shapes, "Delete Main hall?").is_some());
    assert!(ShapeProbe::text_rect(&session.shapes, "East wing (carries Main hall)").is_some());
    assert!(ShapeProbe::text_rect(&session.shapes, "Lighthouse (carries Main hall)").is_some());
    Scenes::click_last(&mut session, "Delete");
    assert_eq!(session.actions, [LibraryAction::DeleteStructure(StructureId(1))]);
    assert!(session.state.dialog.is_none());
}

#[test]
fn delete_cancel_pushes_nothing() {
    let mut session = Scenes::structure_page(SnapshotFixture::library());
    Scenes::click_first(&mut session, "Delete");
    Scenes::click_last(&mut session, "Cancel");
    assert!(session.actions.is_empty(), "{:?}", session.actions);
    assert!(session.state.dialog.is_none());
}

#[test]
fn place_marker_disabled_outside_a_world() {
    let mut snapshot = SnapshotFixture::library();
    snapshot.in_world = false;
    let mut session = Scenes::structure_page(snapshot);
    Scenes::click_first(&mut session, "Place marker here");
    assert!(session.state.dialog.is_none(), "the button is disabled outside a world");
    assert!(session.actions.is_empty(), "{:?}", session.actions);
}

#[test]
fn place_marker_disabled_when_markers_are_read_only() {
    let mut snapshot = SnapshotFixture::library();
    snapshot.markers_read_only = true;
    let mut session = Scenes::structure_page(snapshot);
    Scenes::click_first(&mut session, "Place marker here");
    assert!(session.state.dialog.is_none());
    assert!(session.actions.is_empty(), "{:?}", session.actions);
}

#[test]
fn place_marker_pushes_the_target_and_its_name_at_once() {
    let mut session = Scenes::structure_page(SnapshotFixture::library());
    Scenes::click_first(&mut session, "Place marker here");
    let expected =
        LibraryAction::PlaceMarker { name: "Main hall".to_owned(), target: MarkerTarget::Structure(StructureId(1)) };
    assert_eq!(session.actions, [expected]);
    assert!(session.state.dialog.is_none(), "no dialog opens");
}

#[test]
fn rename_dialog_refuses_empty_name() {
    let mut session = Scenes::structure_page(SnapshotFixture::library());
    let dialog = RenameDialog::new(DialogItem::Structure(StructureId(1)), String::new());
    Scenes::open_dialog(&mut session, Dialog::Rename(dialog));
    Scenes::click_last(&mut session, "Rename");
    assert!(session.actions.is_empty(), "{:?}", session.actions);
    assert!(session.state.dialog.is_some(), "the dialog stays open");
}

#[test]
fn rename_dialog_pushes_the_new_name() {
    let mut session = Scenes::structure_page(SnapshotFixture::library());
    Scenes::click_first(&mut session, "Rename");
    Scenes::click_last(&mut session, "Rename");
    assert_eq!(session.actions, [LibraryAction::RenameStructure(StructureId(1), "Main hall".to_owned())]);
}

#[test]
fn rename_dialog_refuses_long_name() {
    let mut session = Scenes::structure_page(SnapshotFixture::library());
    let dialog = RenameDialog::new(DialogItem::Structure(StructureId(1)), "a".repeat(65));
    Scenes::open_dialog(&mut session, Dialog::Rename(dialog));
    Scenes::click_last(&mut session, "Rename");
    assert!(session.actions.is_empty(), "{:?}", session.actions);
    assert!(session.state.dialog.is_some());
}

#[test]
fn rename_dialog_accepts_a_name_of_64_characters() {
    let mut session = Scenes::structure_page(SnapshotFixture::library());
    let name = "a".repeat(64);
    let dialog = RenameDialog::new(DialogItem::Structure(StructureId(1)), name.clone());
    Scenes::open_dialog(&mut session, Dialog::Rename(dialog));
    Scenes::click_last(&mut session, "Rename");
    assert_eq!(session.actions, [LibraryAction::RenameStructure(StructureId(1), name)]);
}

#[test]
fn rename_marker_dialog_counts_characters_not_bytes() {
    let mut session = Scenes::structure_page(SnapshotFixture::library());
    let long = RenameDialog::new(DialogItem::Marker(MarkerId(1)), "é".repeat(65));
    Scenes::open_dialog(&mut session, Dialog::Rename(long));
    Scenes::click_last(&mut session, "Rename");
    assert!(session.actions.is_empty(), "{:?}", session.actions);
    let name = "é".repeat(64);
    let fits = RenameDialog::new(DialogItem::Marker(MarkerId(1)), name.clone());
    Scenes::open_dialog(&mut session, Dialog::Rename(fits));
    Scenes::click_last(&mut session, "Rename");
    assert_eq!(session.actions, [LibraryAction::RenameMarker(MarkerId(1), name)]);
}

#[test]
fn new_project_dialog_pushes_the_typed_name() {
    let mut session = ViewSession::new(SnapshotFixture::library());
    Scenes::click_first(&mut session, "New project");
    Scenes::click_last(&mut session, "Name");
    session.type_text("Barn");
    Scenes::click_last(&mut session, "Create");
    assert_eq!(session.actions, [LibraryAction::CreateProject("Barn".to_owned())]);
    assert!(session.state.dialog.is_none());
}

#[test]
fn new_project_dialog_refuses_long_name() {
    let mut session = ViewSession::new(SnapshotFixture::library());
    Scenes::click_first(&mut session, "New project");
    Scenes::click_last(&mut session, "Name");
    session.type_text(&"b".repeat(65));
    Scenes::click_last(&mut session, "Create");
    assert!(session.actions.is_empty(), "{:?}", session.actions);
    assert!(session.state.dialog.is_some());
}

#[test]
fn escape_closes_a_dialog_without_an_action() {
    let mut session = Scenes::structure_page(SnapshotFixture::library());
    Scenes::open_delete(&mut session);
    session.press(Key::Escape);
    assert!(session.state.dialog.is_none());
    assert!(session.actions.is_empty(), "{:?}", session.actions);
}

#[test]
fn a_scrim_click_closes_a_dialog_without_an_action() {
    let mut session = Scenes::structure_page(SnapshotFixture::library());
    Scenes::open_delete(&mut session);
    session.click(Pos2::new(8.0, 8.0));
    assert!(session.state.dialog.is_none());
    assert!(session.actions.is_empty(), "{:?}", session.actions);
}

#[test]
fn the_close_button_closes_a_dialog_without_an_action() {
    let mut session = Scenes::structure_page(SnapshotFixture::library());
    Scenes::open_delete(&mut session);
    // the dialog's close button is painted after the panel's
    Scenes::click_last(&mut session, "\u{E8BB}");
    assert!(session.state.dialog.is_none());
    assert!(session.actions.is_empty(), "{:?}", session.actions);
}

#[test]
fn a_click_inside_the_card_keeps_the_dialog_open() {
    let mut session = Scenes::structure_page(SnapshotFixture::library());
    Scenes::open_delete(&mut session);
    Scenes::click_last(&mut session, "East wing (carries");
    assert!(session.state.dialog.is_some());
    assert!(session.actions.is_empty());
}

#[test]
fn move_dialog_moves_to_unsorted() {
    let mut session = Scenes::structure_page(SnapshotFixture::library());
    Scenes::click_first(&mut session, "Move to project");
    Scenes::choose(&mut session, "Willow Manor", "Unsorted");
    Scenes::click_last(&mut session, "Move");
    assert_eq!(session.actions, [LibraryAction::MoveStructure(StructureId(1), None)]);
}

#[test]
fn move_dialog_moves_to_the_chosen_project() {
    let mut session = Scenes::structure_page(SnapshotFixture::library());
    Scenes::click_first(&mut session, "Move to project");
    Scenes::choose(&mut session, "Willow Manor", "Harbour");
    Scenes::click_last(&mut session, "Move");
    assert_eq!(session.actions, [LibraryAction::MoveStructure(StructureId(1), Some(ProjectId(1)))]);
}

#[test]
fn a_changed_snapshot_does_not_retarget_a_move() {
    let mut session = Scenes::structure_page(SnapshotFixture::library());
    Scenes::click_first(&mut session, "Move to project");
    Scenes::choose(&mut session, "Willow Manor", "Harbour");
    // a project is added in front of the list while the dialog is open
    session.snapshot.projects.insert(0, SnapshotFixture::project(9, "Aardvark", Vec::new()));
    session.frame(Vec::new());
    Scenes::click_last(&mut session, "Move");
    assert_eq!(session.actions, [LibraryAction::MoveStructure(StructureId(1), Some(ProjectId(1)))]);
}

#[test]
fn assign_dialog_from_a_structure_picks_a_marker() {
    let mut session = Scenes::structure_page(SnapshotFixture::library());
    Scenes::click_first(&mut session, "Assign to marker");
    Scenes::choose(&mut session, "Choose one", "Lighthouse");
    Scenes::click_last(&mut session, "Assign");
    let target = MarkerTarget::Structure(StructureId(1));
    assert_eq!(session.actions, [LibraryAction::AssignMarker(MarkerId(2), target)]);
}

#[test]
fn assign_dialog_from_a_project_picks_a_marker() {
    let mut snapshot = SnapshotFixture::library();
    // a normal project's card is for its selected structure; a big build's is for the project
    snapshot.projects[0].kind = ProjectKind::BigBuild;
    let mut session = ViewSession::new(snapshot);
    session.state.page = Page::Project(ProjectId(2));
    session.frame(Vec::new());
    Scenes::click_first(&mut session, "Assign to marker");
    Scenes::choose(&mut session, "Choose one", "East wing");
    Scenes::click_last(&mut session, "Assign");
    let target = MarkerTarget::Project(ProjectId(2));
    assert_eq!(session.actions, [LibraryAction::AssignMarker(MarkerId(1), target)]);
}

#[test]
fn assign_dialog_from_a_marker_picks_a_structure() {
    let mut session = ViewSession::new(SnapshotFixture::library());
    let dialog = AssignDialog::new(AssignSubject::Marker(MarkerId(2)));
    Scenes::open_dialog(&mut session, Dialog::Assign(dialog));
    Scenes::choose(&mut session, "Choose one", "Harbour west");
    Scenes::click_last(&mut session, "Assign");
    let target = MarkerTarget::Structure(StructureId(3));
    assert_eq!(session.actions, [LibraryAction::AssignMarker(MarkerId(2), target)]);
}

#[test]
fn assign_dialog_from_a_marker_picks_a_project() {
    let mut session = ViewSession::new(SnapshotFixture::library());
    let dialog = AssignDialog::new(AssignSubject::Marker(MarkerId(2)));
    Scenes::open_dialog(&mut session, Dialog::Assign(dialog));
    Scenes::click_last(&mut session, "Choose one");
    Scenes::click_exact(&mut session, "Harbour");
    Scenes::click_last(&mut session, "Assign");
    let target = MarkerTarget::Project(ProjectId(1));
    assert_eq!(session.actions, [LibraryAction::AssignMarker(MarkerId(2), target)]);
}

// The reason a disabled "Place marker here" gives in its tooltip, read after the pointer rests on the button.
struct PlaceReason;

impl PlaceReason {
    const NOT_IN_WORLD: &str = "Join a world to place a marker";
    const READ_ONLY: &str = "The marker file of this world could not be read";

    fn hovered(snapshot: LibrarySnapshot) -> ViewSession {
        let mut session = Scenes::structure_page(snapshot);
        let spot = ShapeProbe::text_centre(&session.shapes, "Place marker here").expect("button is painted");
        session.hover(spot);
        session
    }

    fn count(session: &ViewSession, text: &str) -> usize {
        ShapeProbe::exact_rects(&session.shapes, text).len()
    }
}

#[test]
fn place_marker_outside_a_world_says_to_join_one() {
    let mut snapshot = SnapshotFixture::library();
    snapshot.in_world = false;
    let session = PlaceReason::hovered(snapshot);
    assert_eq!(PlaceReason::count(&session, PlaceReason::NOT_IN_WORLD), 1);
}

#[test]
fn place_marker_with_read_only_markers_names_the_marker_file() {
    let mut snapshot = SnapshotFixture::library();
    snapshot.markers_read_only = true;
    let session = PlaceReason::hovered(snapshot);
    // the banner over the page and the tooltip
    assert_eq!(PlaceReason::count(&session, PlaceReason::READ_ONLY), 2);
    assert_eq!(PlaceReason::count(&session, PlaceReason::NOT_IN_WORLD), 0);
}

#[test]
fn not_in_a_world_comes_before_read_only_markers() {
    let mut snapshot = SnapshotFixture::library();
    snapshot.in_world = false;
    snapshot.markers_read_only = true;
    let session = PlaceReason::hovered(snapshot);
    assert_eq!(PlaceReason::count(&session, PlaceReason::NOT_IN_WORLD), 1);
    // only the banner
    assert_eq!(PlaceReason::count(&session, PlaceReason::READ_ONLY), 1);
}
