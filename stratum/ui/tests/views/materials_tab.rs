use egui::OutputCommand;
use stratum_core::structs::action::{LibraryAction, MaterialScope};
use stratum_core::structs::library::{ProjectId, ProjectKind, StructureId};
use stratum_ui::state::{Page, ProjectTab, StructureTab};

use crate::support::{ShapeProbe, SnapshotFixture, ViewSession};

// Willow Manor, the project of the Project materials artboard.
const WILLOW: ProjectId = ProjectId(2);

struct Sessions;

impl Sessions {
    fn materials_tab(snapshot: stratum_core::structs::snapshot::LibrarySnapshot) -> ViewSession {
        let mut session = ViewSession::new(snapshot);
        session.state.page = Page::Project(WILLOW);
        session.state.project_tab = ProjectTab::Materials;
        for _ in 0..3 {
            session.frame(Vec::new());
        }
        session
    }
}

#[test]
fn materials_tab_requests_load_once() {
    let mut session = Sessions::materials_tab(SnapshotFixture::library());
    assert_eq!(session.actions, [LibraryAction::LoadMaterials(MaterialScope::Project(WILLOW))]);
    for _ in 0..5 {
        session.frame(Vec::new());
    }
    assert_eq!(session.actions.len(), 1, "{:?}", session.actions);
}

#[test]
fn materials_tab_does_not_request_a_list_that_arrived() {
    let session = Sessions::materials_tab(SnapshotFixture::library_with_materials());
    assert!(session.actions.is_empty(), "{:?}", session.actions);
}

#[test]
fn materials_tab_does_not_request_while_loading() {
    let mut snapshot = SnapshotFixture::library();
    snapshot.materials_loading = Some(MaterialScope::Project(WILLOW));
    let session = Sessions::materials_tab(snapshot);
    assert!(session.actions.is_empty(), "{:?}", session.actions);
}

#[test]
fn materials_tab_asks_again_after_the_page_was_left() {
    let mut session = Sessions::materials_tab(SnapshotFixture::library());
    session.actions.clear();
    session.state.page = Page::All;
    session.frame(Vec::new());
    session.state.page = Page::Project(WILLOW);
    session.frame(Vec::new());
    assert_eq!(session.actions, [LibraryAction::LoadMaterials(MaterialScope::Project(WILLOW))]);
}

#[test]
fn loading_shows_a_loading_state_and_no_table() {
    let mut snapshot = SnapshotFixture::library();
    snapshot.materials_loading = Some(MaterialScope::Project(WILLOW));
    let session = Sessions::materials_tab(snapshot);
    assert!(ShapeProbe::text_rect(&session.shapes, "Reading the structures").is_some());
    assert!(ShapeProbe::text_rect(&session.shapes, "ITEM").is_none());
}

#[test]
fn arrived_list_shows_the_table() {
    let session = Sessions::materials_tab(SnapshotFixture::library_with_materials());
    for text in ["ITEM", "Stone Bricks", "3,412", "53 stacks + 20", "All 1 structure", "4 items", "Copy project list"] {
        assert!(ShapeProbe::text_rect(&session.shapes, text).is_some(), "{text} is not painted");
    }
}

#[test]
fn copy_button_emits_copy_text() {
    let mut session = Sessions::materials_tab(SnapshotFixture::library_with_materials());
    let spot = ShapeProbe::text_centre(&session.shapes, "Copy project list").expect("copy button is painted");
    session.click(spot);
    let copied: Vec<&String> = session
        .commands
        .iter()
        .filter_map(|command| match command {
            OutputCommand::CopyText(text) => Some(text),
            _ => None,
        })
        .collect();
    assert_eq!(copied.len(), 1, "{:?}", session.commands);
    assert!(copied[0].starts_with("Willow Manor\r\nStone Bricks: 3,412 (53 stacks + 20)\r\n"), "{:?}", copied[0]);
    assert!(!session.state.copy_too_large);
}

#[test]
fn copy_follows_the_filter() {
    let mut session = Sessions::materials_tab(SnapshotFixture::library_with_materials());
    session.state.filter = "lantern".to_owned();
    session.frame(Vec::new());
    session.frame(Vec::new());
    let spot = ShapeProbe::text_centre(&session.shapes, "Copy project list").expect("copy button is painted");
    session.click(spot);
    assert!(session.commands.contains(&OutputCommand::CopyText("Willow Manor\r\nLantern: 42 (42)\r\n".to_owned())));
}

#[test]
fn a_failed_load_shows_an_error_and_retry_asks_again() {
    let scope = MaterialScope::Project(WILLOW);
    let mut session = Sessions::materials_tab(SnapshotFixture::library());
    assert_eq!(session.actions, [LibraryAction::LoadMaterials(scope)]);
    session.snapshot.materials_loading = Some(scope);
    session.snapshot.revision += 1;
    session.frame(Vec::new());
    assert!(ShapeProbe::text_rect(&session.shapes, "Reading the structures").is_some());
    session.snapshot.materials_loading = None;
    session.snapshot.revision += 1;
    session.frame(Vec::new());
    session.frame(Vec::new());
    assert!(ShapeProbe::text_rect(&session.shapes, "The item list could not be read").is_some());
    assert!(ShapeProbe::text_rect(&session.shapes, "Reading the structures").is_none());
    assert_eq!(session.actions.len(), 1, "no automatic retry: {:?}", session.actions);
    session.actions.clear();
    let spot = ShapeProbe::text_centre(&session.shapes, "Try again").expect("retry button is painted");
    session.click(spot);
    assert_eq!(session.actions, [LibraryAction::LoadMaterials(scope)]);
}

#[test]
fn a_load_refused_before_it_started_shows_the_error() {
    let scope = MaterialScope::Project(WILLOW);
    let mut session = Sessions::materials_tab(SnapshotFixture::library());
    assert_eq!(session.actions, [LibraryAction::LoadMaterials(scope)]);
    session.snapshot.materials_failed = Some(scope);
    session.snapshot.revision += 1;
    session.frame(Vec::new());
    session.frame(Vec::new());
    assert!(ShapeProbe::text_rect(&session.shapes, "The item list could not be read").is_some());
    assert!(ShapeProbe::text_rect(&session.shapes, "Reading the structures").is_none());
    session.actions.clear();
    let spot = ShapeProbe::text_centre(&session.shapes, "Try again").expect("retry button is painted");
    session.click(spot);
    assert_eq!(session.actions, [LibraryAction::LoadMaterials(scope)]);
}

#[test]
fn changing_the_filter_clears_the_too_large_notice() {
    let mut session = Sessions::materials_tab(SnapshotFixture::library_with_materials());
    session.state.copy_too_large = true;
    session.frame(Vec::new());
    assert!(ShapeProbe::text_rect(&session.shapes, "The list is too large to copy").is_some());
    session.state.filter = "lan".to_owned();
    session.frame(Vec::new());
    assert!(!session.state.copy_too_large);
}

#[test]
fn summary_has_a_singular_and_a_plural_form() {
    let session = Sessions::materials_tab(SnapshotFixture::library_with_materials());
    assert!(ShapeProbe::text_rect(&session.shapes, "4 items · ").is_some());
    let mut snapshot = SnapshotFixture::library_with_materials();
    if let Some((_, list)) = snapshot.materials.as_mut() {
        *list =
            stratum_core::structs::material::MaterialList::new(vec![SnapshotFixture::line("minecraft:lantern", 3, 64)]);
    }
    let session = Sessions::materials_tab(snapshot);
    assert!(ShapeProbe::text_rect(&session.shapes, "1 item · ").is_some());
}

#[test]
fn a_filter_with_no_match_says_so() {
    let mut session = Sessions::materials_tab(SnapshotFixture::library_with_materials());
    session.state.materials_shown = Some(MaterialScope::Project(WILLOW));
    session.state.filter = "obsidian".to_owned();
    session.frame(Vec::new());
    session.frame(Vec::new());
    assert!(ShapeProbe::text_rect(&session.shapes, "No item matches the filter").is_some());
    assert!(session.commands.is_empty());
}

#[test]
fn project_page_lists_structures_with_size_and_blocks() {
    let mut session = ViewSession::new(SnapshotFixture::library());
    session.state.page = Page::Project(WILLOW);
    session.frame(Vec::new());
    session.frame(Vec::new());
    assert!(ShapeProbe::text_rect(&session.shapes, "Main hall").is_some());
    assert!(ShapeProbe::text_rect(&session.shapes, "32 × 18 × 24 · 4,212 blocks").is_some());
    assert!(ShapeProbe::text_rect(&session.shapes, "1 structure").is_some());
}

#[test]
fn double_clicking_a_structure_row_opens_the_structure_page() {
    let mut session = ViewSession::new(SnapshotFixture::library());
    session.state.page = Page::Project(WILLOW);
    session.frame(Vec::new());
    session.frame(Vec::new());
    let spot = ShapeProbe::text_centre(&session.shapes, "32 × 18 × 24").expect("structure row is painted");
    session.double_click(spot);
    assert_eq!(session.state.page, Page::Structure(StructureId(1)));
}

#[test]
fn structure_page_goes_back_to_its_project() {
    let mut session = ViewSession::new(SnapshotFixture::library());
    session.state.page = Page::Structure(StructureId(1));
    session.state.structure_tab = StructureTab::Facts;
    session.frame(Vec::new());
    session.frame(Vec::new());
    assert!(ShapeProbe::text_rect(&session.shapes, "In Willow Manor").is_some());
    let spot = ShapeProbe::text_centre(&session.shapes, "Back to Willow Manor").expect("back button is painted");
    session.click(spot);
    assert_eq!(session.state.page, Page::Project(WILLOW));
}

#[test]
fn structure_facts_show_the_values() {
    let mut session = ViewSession::new(SnapshotFixture::library());
    session.state.page = Page::Structure(StructureId(1));
    session.frame(Vec::new());
    session.frame(Vec::new());
    for text in ["Preview", "32 × 18 × 24", "18 layers", "4,212 blocks", "no saved origin"] {
        assert!(ShapeProbe::text_rect(&session.shapes, text).is_some(), "{text} is not painted");
    }
}

#[test]
fn big_build_switch_pushes_set_project_kind() {
    let mut session = ViewSession::new(SnapshotFixture::library());
    session.state.page = Page::Project(WILLOW);
    session.frame(Vec::new());
    session.frame(Vec::new());
    let label = ShapeProbe::text_rect(&session.shapes, "Big build").expect("switch label is painted");
    // Paper: Big build switch, 12 gap between the label column and the 34 wide switch
    let spot = egui::pos2(label.right() + 12.0 + 17.0, label.center().y + 8.0);
    session.click(spot);
    assert_eq!(session.actions, [LibraryAction::SetProjectKind(WILLOW, ProjectKind::BigBuild)]);
}
