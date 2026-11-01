use amml_ui::tokens::{ColorToken, SpacingToken, color, spacing};
use egui::{Rect, vec2};
use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::layout::LayoutIssue;
use stratum_core::structs::library::{ProjectId, ProjectKind, StructureId};
use stratum_core::structs::marker::MarkerTarget;
use stratum_core::structs::snapshot::LibrarySnapshot;
use stratum_core::structs::structure::StructureSize;
use stratum_ui::state::Page;

use crate::support::{RowSpots, ShapeProbe, SnapshotFixture, ViewSession};

// A big build project "Harbour" (id 7) with one piece, whose layout failed with `issue`.
struct Scenes;

impl Scenes {
    fn session(issue: LayoutIssue) -> ViewSession {
        let piece = SnapshotFixture::structure(1, "Harbour west", Some(7));
        let mut project = SnapshotFixture::project(7, "Harbour", vec![piece]);
        project.kind = ProjectKind::BigBuild;
        let mut snapshot = LibrarySnapshot::empty();
        snapshot.revision = 1;
        snapshot.projects = vec![project];
        snapshot.all_count = 1;
        snapshot.layouts = vec![(ProjectId(7), Err(issue))];
        let mut session = ViewSession::new(snapshot);
        session.state.page = Page::Project(ProjectId(7));
        session.frame(Vec::new());
        session
    }
}

impl Scenes {
    // A normal project "Harbour" (id 7) with `pieces` structures; the page is open.
    fn plain(pieces: u64) -> ViewSession {
        let rows = (1..=pieces).map(|id| SnapshotFixture::structure(id, "Harbour west", Some(7))).collect();
        let project = SnapshotFixture::project(7, "Harbour", rows);
        let mut snapshot = SnapshotFixture::library();
        snapshot.projects.push(project);
        let mut session = ViewSession::new(snapshot);
        session.state.page = Page::Project(ProjectId(7));
        session.frame(Vec::new());
        session
    }
}

#[test]
fn project_page_import_targets_its_project() {
    let mut session = Scenes::plain(1);
    let rects = ShapeProbe::exact_rects(&session.shapes, "Import .mcstructure");
    assert_eq!(rects.len(), 2, "sidebar and title row each paint the button");
    let title_row = rects.iter().max_by(|a, b| a.min.x.total_cmp(&b.min.x)).expect("two buttons").center();
    session.click(title_row);
    assert_eq!(session.actions, [LibraryAction::RequestFilePicker(Some(ProjectId(7)))]);
}

#[test]
fn empty_project_shows_the_first_step() {
    let mut session = Scenes::plain(0);
    assert!(ShapeProbe::text_rect(&session.shapes, "No structures yet").is_some());
    assert!(ShapeProbe::text_rect(&session.shapes, "Import .mcstructure files into this project").is_some());
    assert!(ShapeProbe::text_rect(&session.shapes, "Place marker here").is_none());
    assert!(ShapeProbe::text_rect(&session.shapes, "Assign to marker").is_none());
    assert!(ShapeProbe::text_rect(&session.shapes, "Big build").is_none());
    let rects = ShapeProbe::exact_rects(&session.shapes, "Import .mcstructure");
    assert_eq!(rects.len(), 2, "the sidebar button and the empty state button");
    let button = rects.iter().max_by(|a, b| a.min.x.total_cmp(&b.min.x)).expect("two buttons").center();
    session.click(button);
    assert_eq!(session.actions, [LibraryAction::RequestFilePicker(Some(ProjectId(7)))]);
}

// A library with one project and no structures at all, on `page`.
struct EmptyLibrary;

impl EmptyLibrary {
    fn on(page: Page) -> ViewSession {
        let mut snapshot = LibrarySnapshot::empty();
        snapshot.revision = 1;
        snapshot.projects = vec![SnapshotFixture::project(7, "Harbour", Vec::new())];
        let mut session = ViewSession::new(snapshot);
        session.state.page = page;
        session.frame(Vec::new());
        session
    }

    fn click_empty_state_import(session: &mut ViewSession) {
        let rects = ShapeProbe::exact_rects(&session.shapes, "Import .mcstructure");
        assert_eq!(rects.len(), 2, "the sidebar button and the empty state button");
        let button = rects.iter().max_by(|a, b| a.min.x.total_cmp(&b.min.x)).expect("two buttons").center();
        session.click(button);
    }
}

#[test]
fn a_project_page_of_an_empty_library_shows_its_own_empty_state_and_imports_into_it() {
    let mut session = EmptyLibrary::on(Page::Project(ProjectId(7)));
    assert!(ShapeProbe::text_rect(&session.shapes, "No structures yet").is_some());
    assert!(ShapeProbe::text_rect(&session.shapes, "Your library is empty").is_none());

    EmptyLibrary::click_empty_state_import(&mut session);

    assert_eq!(session.actions, [LibraryAction::RequestFilePicker(Some(ProjectId(7)))]);
}

#[test]
fn the_library_empty_state_on_all_and_unsorted_imports_to_unsorted() {
    for page in [Page::All, Page::Unsorted] {
        let mut session = EmptyLibrary::on(page);
        assert!(ShapeProbe::text_rect(&session.shapes, "Your library is empty").is_some());

        EmptyLibrary::click_empty_state_import(&mut session);

        assert_eq!(session.actions, [LibraryAction::RequestFilePicker(None)]);
    }
}

#[test]
fn empty_project_hides_the_tabs() {
    let session = Scenes::plain(0);
    assert!(ShapeProbe::text_rect(&session.shapes, "Materials").is_none());
}

#[test]
fn project_with_structures_shows_no_empty_state() {
    let session = Scenes::plain(1);
    assert!(ShapeProbe::text_rect(&session.shapes, "No structures yet").is_none());
    assert!(ShapeProbe::text_rect(&session.shapes, "Big build").is_some());
}

#[test]
fn a_missing_origin_banner_does_not_talk_about_overlap() {
    let session = Scenes::session(LayoutIssue::MissingOrigin(vec![StructureId(1)]));
    assert!(ShapeProbe::text_rect(&session.shapes, "no saved position").is_some());
    assert!(ShapeProbe::text_rect(&session.shapes, "overlap").is_none());
}

#[test]
fn an_overlap_banner_talks_about_overlap() {
    let session = Scenes::session(LayoutIssue::Overlap(StructureId(1), StructureId(2)));
    assert!(ShapeProbe::text_rect(&session.shapes, "overlap").is_some());
}

#[test]
fn an_incomplete_grid_banner_names_the_grid() {
    let session = Scenes::session(LayoutIssue::GridIncomplete(vec![StructureId(1)]));
    assert!(ShapeProbe::text_rect(&session.shapes, "not on the grid").is_some());
}

#[test]
fn a_build_with_no_pieces_has_its_own_banner() {
    let session = Scenes::session(LayoutIssue::NoPieces);
    assert!(ShapeProbe::text_rect(&session.shapes, "No pieces to line up").is_some());
}

#[test]
fn a_build_that_is_too_large_has_its_own_banner() {
    let session = Scenes::session(LayoutIssue::TooLarge);
    assert!(ShapeProbe::text_rect(&session.shapes, "The build is too large").is_some());
}

#[test]
fn title_row_import_sits_left_of_the_big_build_switch_with_the_large_gap() {
    let session = Scenes::plain(1);
    let rects = ShapeProbe::exact_rects(&session.shapes, "Import .mcstructure");
    let button = rects.iter().max_by(|a, b| a.min.x.total_cmp(&b.min.x)).expect("title row button");
    let label = ShapeProbe::text_rect(&session.shapes, "Big build").expect("switch label");
    let help = ShapeProbe::text_rect(&session.shapes, "Pieces show as one build").expect("switch help");
    let switch_left = label.min.x.min(help.min.x);
    let button_right = button.max.x + <spacing::ButtonX as SpacingToken>::VALUE;
    assert!(button_right <= switch_left, "button {button_right} is not left of the switch {switch_left}");
    let gap = switch_left - button_right;
    let want = <spacing::L as SpacingToken>::VALUE;
    assert!((gap - want).abs() < 1.0, "gap {gap}, want {want}");
}

#[test]
fn big_build_rows_have_no_place_action() {
    let mut session = Scenes::session(LayoutIssue::TooLarge);
    session.snapshot.in_world = true;
    session.frame(Vec::new());
    let spot = RowSpots::structure_action(&mut session, "Harbour west").expect("row is hovered");
    let slot = Rect::from_center_size(spot, vec2(28.0, 28.0));
    assert_eq!(ShapeProbe::count_inside(&session.shapes, slot), 0, "nothing is painted where the button would be");
    session.click(spot);
    assert_eq!(session.state.dialog, None);
    assert_eq!(session.state.page, Page::Structure(StructureId(1)));
}

// A normal project "Willow Manor" (id 7) with "Main hall" (32 x 18 x 24) and "East tower" (9 x 41 x 9).
struct Pair;

impl Pair {
    fn session() -> ViewSession {
        let hall = SnapshotFixture::structure(1, "Main hall", Some(7));
        let mut tower = SnapshotFixture::structure(2, "East tower", Some(7));
        tower.size = StructureSize::new(9, 41, 9);
        tower.block_count = 2018;
        let mut snapshot = SnapshotFixture::library();
        snapshot.projects = vec![SnapshotFixture::project(7, "Willow Manor", vec![hall, tower])];
        let mut session = ViewSession::new(snapshot);
        session.state.page = Page::Project(ProjectId(7));
        session.frame(Vec::new());
        session
    }

    fn shows_hall(session: &ViewSession) -> bool {
        Self::in_card(session, "32 × 18 × 24").is_some()
    }

    fn shows_tower(session: &ViewSession) -> bool {
        Self::in_card(session, "9 × 41 × 9").is_some()
    }

    // The rect of the text that is exactly `text` and lies inside the card.
    fn in_card(session: &ViewSession, text: &str) -> Option<Rect> {
        let card = Self::card(session);
        ShapeProbe::exact_rects(&session.shapes, text).into_iter().find(|rect| card.contains_rect(*rect))
    }

    fn card(session: &ViewSession) -> Rect {
        Self::card_of(session, "Main hall")
    }

    // The raised card: the largest raised fill at the right of the row named `anchor`.
    fn card_of(session: &ViewSession, anchor: &str) -> Rect {
        let list_right = ShapeProbe::exact_rects(&session.shapes, anchor)
            .into_iter()
            .map(|rect| rect.right())
            .fold(f32::MIN, f32::max);
        let raised = <color::BgRaised as ColorToken>::VALUE;
        ShapeProbe::fill_rects(&session.shapes, raised)
            .into_iter()
            .filter(|rect| rect.min.x > list_right)
            .max_by(|a, b| a.height().total_cmp(&b.height()))
            .expect("the card is painted")
    }
}

#[test]
fn a_normal_project_with_no_selection_shows_the_first_structure() {
    let session = Pair::session();
    assert!(Pair::shows_hall(&session));
    assert!(!Pair::shows_tower(&session));
}

#[test]
fn clicking_a_structure_row_selects_it_and_keeps_the_page() {
    let mut session = Pair::session();
    let spot = ShapeProbe::text_centre(&session.shapes, "East tower").expect("row is painted");
    session.click(spot);
    assert_eq!(session.state.page, Page::Project(ProjectId(7)));
    assert_eq!(session.state.selected_structure, Some(StructureId(2)));
    assert!(Pair::shows_tower(&session));
    assert!(!Pair::shows_hall(&session));
}

#[test]
fn a_double_click_on_a_structure_row_opens_its_page() {
    let mut session = Pair::session();
    let spot = ShapeProbe::text_centre(&session.shapes, "East tower").expect("row is painted");
    session.double_click(spot);
    assert_eq!(session.state.page, Page::Structure(StructureId(2)));
}

#[test]
fn the_card_falls_back_to_the_first_structure_when_the_selected_one_is_gone() {
    let mut session = Pair::session();
    let spot = ShapeProbe::text_centre(&session.shapes, "East tower").expect("row is painted");
    session.click(spot);
    assert!(Pair::shows_tower(&session));
    session.snapshot.projects[0].structures.retain(|row| row.id != StructureId(2));
    session.snapshot.revision += 1;
    session.frame(Vec::new());
    session.frame(Vec::new());
    assert!(Pair::shows_hall(&session));
}

#[test]
fn the_card_lists_no_name_row() {
    let session = Pair::session();
    assert!(Pair::in_card(&session, "no saved origin").is_some());
    assert!(ShapeProbe::exact_rects(&session.shapes, "Name").is_empty());
}

#[test]
fn the_card_footer_acts_on_the_selected_structure() {
    let mut session = Pair::session();
    let spot = ShapeProbe::text_centre(&session.shapes, "East tower").expect("row is painted");
    session.click(spot);
    let place = Pair::in_card(&session, "Place marker here").expect("footer is painted").center();
    session.click(place);
    let target = MarkerTarget::Structure(StructureId(2));
    assert_eq!(session.actions, [LibraryAction::PlaceMarker { name: "East tower".to_owned(), target }]);
    assert_eq!(session.state.dialog, None, "no dialog opens");
}

#[test]
fn the_card_footer_sits_at_the_bottom_of_a_normal_project_card() {
    let session = Pair::session();
    let card = Pair::card(&session);
    let place = Pair::in_card(&session, "Place marker here").expect("footer is painted");
    let button_bottom = place.center().y + <spacing::Button as SpacingToken>::VALUE / 2.0;
    let padding = <spacing::L as SpacingToken>::VALUE;
    let gap = card.bottom() - button_bottom;
    assert!((0.0..=padding + 1.0).contains(&gap), "the footer ends {gap} above the card's bottom");
}

#[test]
fn the_card_footer_sits_at_the_bottom_of_a_big_build_card() {
    let session = Scenes::session(LayoutIssue::TooLarge);
    let card = Pair::card_of(&session, "Harbour west");
    let place = ShapeProbe::exact_rects(&session.shapes, "Place marker here")
        .into_iter()
        .find(|rect| card.contains_rect(*rect))
        .expect("footer is painted");
    let button_bottom = place.center().y + <spacing::Button as SpacingToken>::VALUE / 2.0;
    let padding = <spacing::L as SpacingToken>::VALUE;
    let gap = card.bottom() - button_bottom;
    assert!((0.0..=padding + 1.0).contains(&gap), "the footer ends {gap} above the card's bottom");
}

#[test]
fn the_size_box_names_the_three_lengths_of_the_selected_structure() {
    let mut session = Pair::session();
    let spot = ShapeProbe::text_centre(&session.shapes, "East tower").expect("row is painted");
    session.click(spot);
    let card = Pair::card(&session);
    let in_card = |text: &str| {
        ShapeProbe::exact_rects(&session.shapes, text).into_iter().filter(|rect| card.contains_rect(*rect)).count()
    };
    assert_eq!(in_card("9"), 2, "the X and Z lengths are painted in the card");
    assert_eq!(in_card("41"), 1, "the Y length is painted in the card");
    assert_eq!(in_card("41 layers"), 1, "the facts line names the layers");
}

#[test]
fn normal_project_rows_have_a_place_action() {
    let mut session = Pair::session();
    session.snapshot.in_world = true;
    session.snapshot.has_player = true;
    session.frame(Vec::new());
    // the second row is not the selected one, so it paints a hover fill
    let spot = RowSpots::structure_action(&mut session, "East tower").expect("row is hovered");
    let slot = Rect::from_center_size(spot, vec2(28.0, 28.0));
    assert!(ShapeProbe::count_inside(&session.shapes, slot) > 0, "the button is painted");
    session.click(spot);
    let target = MarkerTarget::Structure(StructureId(2));
    assert_eq!(session.actions, [LibraryAction::PlaceMarker { name: "East tower".to_owned(), target }]);
    assert_eq!(session.state.dialog, None, "no dialog opens");
    assert_eq!(session.state.selected_structure, Some(StructureId(1)), "the action does not select the row");
}

// Two projects: "Willow Manor" (7) with "Main hall" and "East tower", and "Harbour" (8) with "Quay" and "Light".
struct Two;

impl Two {
    fn session() -> ViewSession {
        let hall = SnapshotFixture::structure(1, "Main hall", Some(7));
        let tower = SnapshotFixture::structure(2, "East tower", Some(7));
        let quay = SnapshotFixture::structure(3, "Quay", Some(8));
        let light = SnapshotFixture::structure(4, "Light", Some(8));
        let mut snapshot = SnapshotFixture::library();
        snapshot.projects = vec![
            SnapshotFixture::project(7, "Willow Manor", vec![hall, tower]),
            SnapshotFixture::project(8, "Harbour", vec![quay, light]),
        ];
        let mut session = ViewSession::new(snapshot);
        session.state.page = Page::Project(ProjectId(7));
        session.frame(Vec::new());
        session
    }
}

#[test]
fn switching_projects_starts_from_the_first_structure_of_the_new_project() {
    let mut session = Two::session();
    let spot = ShapeProbe::text_centre(&session.shapes, "East tower").expect("row is painted");
    session.click(spot);
    assert_eq!(session.state.selected_structure, Some(StructureId(2)));
    session.state.page = Page::Project(ProjectId(8));
    session.frame(Vec::new());
    session.frame(Vec::new());
    assert_eq!(session.state.selected_structure, Some(StructureId(3)));
}

#[test]
fn a_selected_structure_moved_to_another_project_leaves_the_card_on_the_first_one() {
    let mut session = Two::session();
    let spot = ShapeProbe::text_centre(&session.shapes, "East tower").expect("row is painted");
    session.click(spot);
    let tower = session.snapshot.projects[0].structures.pop().expect("tower");
    session.snapshot.projects[1].structures.push(tower);
    session.snapshot.revision += 1;
    session.frame(Vec::new());
    session.frame(Vec::new());
    assert_eq!(session.state.selected_structure, Some(StructureId(1)));
}

#[test]
fn enter_on_the_selected_row_opens_its_page() {
    // Tab walks the focus through the page; Enter on a row that is already selected opens it. Each try starts from a
    // fresh page, so an Enter on another control cannot move the focus for the next one.
    let opened = (1..80).any(|tabs| {
        let mut session = Pair::session();
        session.snapshot.markers.clear();
        session.frame(Vec::new());
        for _ in 0..tabs {
            session.press(egui::Key::Tab);
        }
        session.press(egui::Key::Enter);
        session.state.page == Page::Structure(StructureId(1))
    });
    assert!(opened, "Enter on the focused, selected row opens its page");
}

#[test]
fn enter_on_a_row_that_is_not_selected_only_selects_it() {
    let mut reached = false;
    for tabs in 1..80 {
        let mut session = Pair::session();
        session.snapshot.markers.clear();
        session.frame(Vec::new());
        for _ in 0..tabs {
            session.press(egui::Key::Tab);
        }
        session.press(egui::Key::Enter);
        assert_ne!(session.state.page, Page::Structure(StructureId(2)), "Enter on the unselected row opened it");
        reached |= session.state.selected_structure == Some(StructureId(2));
    }
    assert!(reached, "Enter on the second row selects it");
}

#[test]
fn an_import_into_a_normal_project_selects_the_picked_row() {
    let mut session = Pair::session();
    let mut pick = stratum_core::structs::snapshot::ImportPick::new(1, Some(ProjectId(7)));
    pick.structures.push(StructureId(2));
    session.snapshot.latest_pick = Some(pick);
    session.snapshot.revision += 1;
    session.frame(Vec::new());
    session.frame(Vec::new());
    assert_eq!(session.state.selected_structure, Some(StructureId(2)));
    assert!(Pair::shows_tower(&session));
}

// A screen too short for the whole card.
struct Short;

impl Short {
    const SIZE: egui::Vec2 = vec2(1920.0, 560.0);

    fn session() -> ViewSession {
        let mut session = Pair::session();
        session.resize(Self::SIZE);
        session.frame(Vec::new());
        session.frame(Vec::new());
        session
    }
}

#[test]
fn a_card_taller_than_the_room_keeps_its_footer_under_the_facts() {
    let session = Short::session();
    let footer = Pair::in_card(&session, "Place marker here").expect("footer is painted");
    let delete = Pair::in_card(&session, "Delete").expect("actions are painted");
    assert!(footer.top() > delete.bottom(), "the footer {footer:?} overlaps the actions {delete:?}");
}

#[test]
fn a_scroll_offset_does_not_outlive_the_content_that_needed_it() {
    let mut session = Short::session();
    let before = ShapeProbe::text_rect(&session.shapes, "no saved origin").expect("facts are painted").top();
    let card = Pair::card(&session);
    session.scroll(card.min + vec2(40.0, 40.0), vec2(0.0, -400.0));
    let scrolled = ShapeProbe::text_rect(&session.shapes, "no saved origin").expect("facts are painted").top();
    assert!(scrolled < before - 50.0, "the card scrolled from {before} to {scrolled}");

    session.resize(vec2(1920.0, 1080.0));
    for _ in 0..4 {
        session.frame(Vec::new());
    }
    let card = Pair::card(&session);
    let header = Pair::in_card(&session, "no saved origin").expect("facts are painted in the card");
    assert!(header.top() - card.top() < 260.0, "the card is still scrolled: header {header:?}, card {card:?}");
    let footer = Pair::in_card(&session, "Place marker here").expect("footer is painted");
    let button_bottom = footer.center().y + <spacing::Button as SpacingToken>::VALUE / 2.0;
    let gap = card.bottom() - button_bottom;
    assert!((0.0..=<spacing::L as SpacingToken>::VALUE + 1.0).contains(&gap), "footer ends {gap} above the bottom");
}

#[test]
fn the_structure_page_card_keeps_its_footer_inside_the_panel_on_a_short_screen() {
    let mut session = Pair::session();
    session.state.page = Page::Structure(StructureId(1));
    session.resize(Short::SIZE);
    session.frame(Vec::new());
    session.frame(Vec::new());
    let panel = session.panel;
    let footer_of = |session: &ViewSession| {
        ShapeProbe::exact_rects(&session.shapes, "Place marker here")
            .into_iter()
            .max_by(|a, b| a.min.x.total_cmp(&b.min.x))
            .expect("footer is painted")
    };
    let over_card = ShapeProbe::text_centre(&session.shapes, "no saved origin").expect("facts are painted");
    session.scroll(over_card, vec2(0.0, -600.0));
    let footer = footer_of(&session);
    assert!(footer.bottom() <= panel.bottom(), "the footer {footer:?} is outside the panel {panel:?}");
    assert!(footer.top() >= panel.top());
}
