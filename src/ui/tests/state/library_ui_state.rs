use stratum_core::structs::library::{ProjectId, StructureId};
use stratum_core::structs::marker::MarkerTarget;
use stratum_ui::state::{LibraryUiState, Page, ProjectTab, StructureTab};

use crate::support::SnapshotFixture;

#[test]
fn page_of_a_deleted_project_falls_back_to_all() {
    let mut state = LibraryUiState::new();
    state.page = Page::Project(ProjectId(99));
    state.reconcile(&SnapshotFixture::library());
    assert_eq!(state.page, Page::All);
}

#[test]
fn page_of_a_kept_project_or_structure_stays() {
    let snapshot = SnapshotFixture::library();
    let mut state = LibraryUiState::new();
    state.page = Page::Project(ProjectId(1));
    state.reconcile(&snapshot);
    assert_eq!(state.page, Page::Project(ProjectId(1)));
    state.page = Page::Structure(StructureId(4));
    state.reconcile(&snapshot);
    assert_eq!(state.page, Page::Structure(StructureId(4)));
    state.page = Page::Structure(StructureId(77));
    state.reconcile(&snapshot);
    assert_eq!(state.page, Page::All);
}

#[test]
fn open_materials_for_a_project_target() {
    let mut state = LibraryUiState::new();
    state.open_materials(MarkerTarget::Project(ProjectId(2)));
    assert_eq!(state.page, Page::Project(ProjectId(2)));
    assert_eq!(state.project_tab, ProjectTab::Materials);
}

#[test]
fn open_materials_for_a_structure_target() {
    let mut state = LibraryUiState::new();
    state.open_materials(MarkerTarget::Structure(StructureId(4)));
    assert_eq!(state.page, Page::Structure(StructureId(4)));
    assert_eq!(state.structure_tab, StructureTab::Materials);
}

#[test]
fn shows_target_on_any_tab() {
    let mut state = LibraryUiState::new();
    let project = MarkerTarget::Project(ProjectId(2));
    let structure = MarkerTarget::Structure(StructureId(4));
    assert!(!state.shows_target(project));
    state.page = Page::Project(ProjectId(2));
    state.project_tab = ProjectTab::Structures;
    assert!(state.shows_target(project));
    state.project_tab = ProjectTab::Materials;
    assert!(state.shows_target(project));
    assert!(!state.shows_target(structure));
    state.page = Page::Structure(StructureId(4));
    state.structure_tab = StructureTab::Facts;
    assert!(state.shows_target(structure));
    assert!(!state.shows_target(project));
    state.page = Page::Structure(StructureId(5));
    assert!(!state.shows_target(structure));
}

#[test]
fn reconcile_clears_a_selection_that_is_gone() {
    let snapshot = SnapshotFixture::library();
    let mut state = LibraryUiState::new();
    state.selected_structure = Some(StructureId(77));
    state.reconcile(&snapshot);
    assert_eq!(state.selected_structure, None);
    state.selected_structure = Some(StructureId(3));
    state.reconcile(&snapshot);
    assert_eq!(state.selected_structure, Some(StructureId(3)));
    state.selected_structure = Some(StructureId(4));
    state.reconcile(&snapshot);
    assert_eq!(state.selected_structure, Some(StructureId(4)));
}
