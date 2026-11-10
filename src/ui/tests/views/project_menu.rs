use egui::{Pos2, Rect};
use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::layout::LayoutIssue;
use stratum_core::structs::library::{ProjectId, ProjectKind, StructureId};
use stratum_core::structs::marker::MarkerTarget;
use stratum_ui::state::{Dialog, DialogItem, Page};
use stratum_ui::views::dialogs::{AssignDialog, AssignSubject, DeleteDialog, RenameDialog};

use crate::support::{ShapeProbe, SnapshotFixture, ViewSession};

// The project menu beside the title of project 7, "Willow Manor", on a normal or a big build page.
struct Menu;

impl Menu {
    // Paper: `Stratum · 18 Project menu`, node "Project header": the gap between the title and the button, 10 px, and the
    // button's side, 28 px.
    const GAP: f32 = 10.0;
    const SIDE: f32 = 28.0;

    fn normal() -> ViewSession {
        let hall = SnapshotFixture::structure(1, "Main hall", Some(7));
        Self::open_page(vec![hall], ProjectKind::Normal)
    }

    fn big_build() -> ViewSession {
        let piece = SnapshotFixture::structure(1, "Main hall", Some(7));
        Self::open_page(vec![piece], ProjectKind::BigBuild)
    }

    fn open_page(rows: Vec<stratum_core::structs::snapshot::StructureRow>, kind: ProjectKind) -> ViewSession {
        let mut project = SnapshotFixture::project(7, "Willow Manor", rows);
        project.kind = kind;
        let mut snapshot = SnapshotFixture::library();
        snapshot.projects = vec![project];
        let mut session = ViewSession::new(snapshot);
        session.state.page = Page::Project(ProjectId(7));
        session.frame(Vec::new());
        session
    }

    // The centre of the chevron button: right of the page title.
    fn button(session: &ViewSession) -> Pos2 {
        let title = ShapeProbe::exact_rects(&session.shapes, "Willow Manor")
            .into_iter()
            .filter(|rect| rect.min.x > 700.0)
            .min_by(|a, b| a.min.y.total_cmp(&b.min.y))
            .expect("the page title is painted");
        Pos2::new(title.right() + Self::GAP + Self::SIDE / 2.0, title.center().y)
    }

    // The centre of a menu item: text left of the card, under the button.
    fn item(session: &ViewSession, label: &str) -> Option<Pos2> {
        let button = Self::button(session);
        ShapeProbe::exact_rects(&session.shapes, label)
            .into_iter()
            .find(|rect| rect.center().y > button.y && rect.max.x < button.x + 220.0)
            .map(|rect: Rect| rect.center())
    }

    fn open(session: &mut ViewSession) {
        let button = Self::button(session);
        session.click(button);
    }
}

#[test]
fn the_chevron_button_opens_the_menu_and_a_second_click_closes_it() {
    let mut session = Menu::normal();
    assert!(Menu::item(&session, "Rename").is_none(), "the menu is closed");
    Menu::open(&mut session);
    for label in ["Rename", "Assign to marker…", "Place marker here", "Delete"] {
        assert!(Menu::item(&session, label).is_some(), "{label} is in the menu");
    }
    Menu::open(&mut session);
    assert!(Menu::item(&session, "Rename").is_none(), "the menu is closed again");
}

#[test]
fn the_menu_is_on_both_page_kinds() {
    for mut session in [Menu::normal(), Menu::big_build()] {
        Menu::open(&mut session);
        assert!(Menu::item(&session, "Rename").is_some());
        assert!(Menu::item(&session, "Delete").is_some());
    }
}

#[test]
fn rename_opens_the_rename_dialog_for_the_project() {
    for mut session in [Menu::normal(), Menu::big_build()] {
        Menu::open(&mut session);
        let item = Menu::item(&session, "Rename").expect("item is painted");
        session.click(item);
        let want = RenameDialog::new(DialogItem::Project(ProjectId(7)), "Willow Manor".to_owned());
        assert_eq!(session.state.dialog, Some(Dialog::Rename(want)));
    }
}

#[test]
fn delete_opens_the_delete_dialog_for_the_project() {
    let mut session = Menu::normal();
    Menu::open(&mut session);
    let item = Menu::item(&session, "Delete").expect("item is painted");
    session.click(item);
    assert_eq!(session.state.dialog, Some(Dialog::Delete(DeleteDialog::new(DialogItem::Project(ProjectId(7))))));
}

#[test]
fn assign_opens_its_dialog_and_place_places_the_project_marker_at_once() {
    let target = MarkerTarget::Project(ProjectId(7));
    let mut session = Menu::normal();
    Menu::open(&mut session);
    let item = Menu::item(&session, "Assign to marker…").expect("item is painted");
    session.click(item);
    assert_eq!(session.state.dialog, Some(Dialog::Assign(AssignDialog::new(AssignSubject::Target(target)))));

    let mut session = Menu::big_build();
    Menu::open(&mut session);
    let item = Menu::item(&session, "Place marker here").expect("item is painted");
    session.click(item);
    assert_eq!(session.actions, [LibraryAction::PlaceMarker { name: "Willow Manor".to_owned(), target }]);
    assert_eq!(session.state.dialog, None, "no dialog opens");
}

#[test]
fn place_is_off_when_markers_are_read_only() {
    let mut session = Menu::normal();
    session.snapshot.markers_read_only = true;
    session.frame(Vec::new());
    Menu::open(&mut session);
    let place = Menu::item(&session, "Place marker here").expect("item is painted");
    session.click(place);
    assert!(session.actions.is_empty(), "{:?}", session.actions);
    assert_eq!(session.state.dialog, None);
}

#[test]
fn a_read_only_library_disables_rename_and_delete_in_the_menu() {
    let mut session = Menu::normal();
    session.snapshot.library_read_only = true;
    session.frame(Vec::new());
    Menu::open(&mut session);
    let delete = Menu::item(&session, "Delete").expect("item is painted");
    session.click(delete);
    assert_eq!(session.state.dialog, None);
    let rename = Menu::item(&session, "Rename").expect("the menu stays open");
    session.click(rename);
    assert_eq!(session.state.dialog, None);
}

#[test]
fn place_waits_for_a_world_and_assign_for_a_marker() {
    let mut session = Menu::normal();
    session.snapshot.in_world = false;
    session.snapshot.markers.clear();
    session.frame(Vec::new());
    Menu::open(&mut session);
    let place = Menu::item(&session, "Place marker here").expect("item is painted");
    session.click(place);
    assert_eq!(session.state.dialog, None);
    assert!(session.actions.is_empty(), "{:?}", session.actions);
    let assign = Menu::item(&session, "Assign to marker…").expect("the menu stays open");
    session.click(assign);
    assert_eq!(session.state.dialog, None);
}

#[test]
fn pieces_that_are_not_placed_hold_back_assign_and_place() {
    let mut session = Menu::big_build();
    session.snapshot.layouts = vec![(ProjectId(7), Err(LayoutIssue::GridIncomplete(vec![StructureId(1)])))];
    session.snapshot.in_world = true;
    session.frame(Vec::new());
    Menu::open(&mut session);
    let place = Menu::item(&session, "Place marker here").expect("item is painted");
    session.click(place);
    assert_eq!(session.state.dialog, None);
    let rename = Menu::item(&session, "Rename").expect("the menu stays open");
    session.click(rename);
    assert!(matches!(session.state.dialog, Some(Dialog::Rename(_))), "rename is not held back");
}

#[test]
fn an_empty_project_has_the_menu_and_can_be_deleted() {
    let mut session = Menu::open_page(Vec::new(), ProjectKind::Normal);
    Menu::open(&mut session);
    let item = Menu::item(&session, "Delete").expect("item is painted");
    session.click(item);
    assert_eq!(session.state.dialog, Some(Dialog::Delete(DeleteDialog::new(DialogItem::Project(ProjectId(7))))));
}

#[test]
fn a_big_build_card_has_no_rename_or_delete_buttons() {
    let session = Menu::big_build();
    let card_left = 1100.0;
    for label in ["Rename", "Delete"] {
        let in_card = ShapeProbe::exact_rects(&session.shapes, label).into_iter().any(|rect| rect.min.x > card_left);
        assert!(!in_card, "{label} is on the card");
    }
    assert!(ShapeProbe::exact_rects(&session.shapes, "Place marker here").iter().any(|rect| rect.min.x > card_left));
}

#[test]
fn an_empty_project_holds_back_place_and_assign() {
    let mut session = Menu::open_page(Vec::new(), ProjectKind::Normal);
    session.snapshot.in_world = true;
    session.frame(Vec::new());
    Menu::open(&mut session);
    for label in ["Place marker here", "Assign to marker…"] {
        let item = Menu::item(&session, label).expect("item is painted");
        session.click(item);
        assert_eq!(session.state.dialog, None, "{label} opened a dialog");
    }
    assert!(Menu::item(&session, "No structures yet").is_some(), "the reason is shown in the menu");
    let rename = Menu::item(&session, "Rename").expect("the menu stays open");
    session.click(rename);
    assert!(matches!(session.state.dialog, Some(Dialog::Rename(_))));
}

#[test]
fn a_long_title_leaves_the_chevron_clear_of_the_big_build_switch() {
    let name = format!("X{}", "x".repeat(59));
    let mut project = SnapshotFixture::project(7, &name, vec![SnapshotFixture::structure(1, "Main hall", Some(7))]);
    project.kind = ProjectKind::BigBuild;
    let mut snapshot = SnapshotFixture::library();
    snapshot.projects = vec![project];
    let mut session = ViewSession::new(snapshot);
    session.state.page = Page::Project(ProjectId(7));
    session.frame(Vec::new());
    let title = ShapeProbe::exact_rects(&session.shapes, &name)
        .into_iter()
        .chain(ShapeProbe::text_rect(&session.shapes, "Xxxx"))
        .filter(|rect| rect.min.x > 700.0)
        .min_by(|a, b| a.min.y.total_cmp(&b.min.y))
        .expect("the title is painted");
    let chevron = Rect::from_center_size(
        Pos2::new(title.right() + Menu::GAP + Menu::SIDE / 2.0, title.center().y),
        egui::vec2(Menu::SIDE, Menu::SIDE),
    );
    let label = ShapeProbe::text_rect(&session.shapes, "Big build").expect("switch label");
    let help = ShapeProbe::text_rect(&session.shapes, "Pieces show as one build").expect("switch help");
    let switch_left = label.min.x.min(help.min.x);
    assert!(chevron.max.x <= switch_left, "chevron {chevron:?} reaches the switch at {switch_left}");
}
