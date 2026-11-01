use stratum_core::config::LibraryConfig;
use stratum_core::errors::MarkerError;
use stratum_core::services::LibraryService;
use stratum_core::structs::library::{Library, ProjectId, StructureId};
use stratum_core::structs::marker::{MarkerTarget, TargetState};

use crate::support::{MarkerFixture, NewStructureFixture};

#[test]
fn place_uses_the_given_origin() {
    let mut svc = MarkerFixture::service(8);
    let id = svc.place("Home", [-5, 64, 300], MarkerTarget::Structure(StructureId(1))).unwrap();
    let marker = svc.markers().iter().find(|m| m.id == id).unwrap();
    assert_eq!(marker.origin, [-5, 64, 300]);
    assert_eq!(marker.name, "Home");
    assert!(marker.view.shown);
}

#[test]
fn only_one_marker_is_active() {
    let mut svc = MarkerFixture::service(8);
    let a = svc.place("A", [0, 0, 0], MarkerTarget::Structure(StructureId(1))).unwrap();
    let b = svc.place("B", [1, 0, 0], MarkerTarget::Structure(StructureId(1))).unwrap();
    svc.activate(a).unwrap();
    assert_eq!(svc.active().unwrap().id, a);
    svc.activate(b).unwrap();
    assert_eq!(svc.active().unwrap().id, b);
    assert_eq!(svc.set().active, Some(b));
}

#[test]
fn deactivate_clears_the_active_marker() {
    let mut svc = MarkerFixture::service(8);
    let a = svc.place("A", [0, 0, 0], MarkerTarget::Structure(StructureId(1))).unwrap();
    svc.activate(a).unwrap();
    svc.deactivate();
    assert!(svc.active().is_none());
    assert_eq!(svc.markers().len(), 1);
}

#[test]
fn activating_an_unknown_marker_fails_and_keeps_the_active_one() {
    let mut svc = MarkerFixture::service(8);
    let a = svc.place("A", [0, 0, 0], MarkerTarget::Structure(StructureId(1))).unwrap();
    svc.activate(a).unwrap();
    let missing = stratum_core::structs::marker::MarkerId(99);
    assert_eq!(svc.activate(missing), Err(MarkerError::NotFound));
    assert_eq!(svc.active().unwrap().id, a);
}

#[test]
fn deleting_the_active_marker_leaves_none_active() {
    let mut svc = MarkerFixture::service(8);
    let a = svc.place("A", [0, 0, 0], MarkerTarget::Structure(StructureId(1))).unwrap();
    let b = svc.place("B", [0, 0, 0], MarkerTarget::Structure(StructureId(1))).unwrap();
    svc.activate(a).unwrap();
    svc.delete(a).unwrap();
    assert!(svc.active().is_none());
    assert_eq!(svc.markers().len(), 1);
    svc.activate(b).unwrap();
    svc.delete(a).unwrap_err();
    assert_eq!(svc.active().unwrap().id, b);
}

#[test]
fn refuses_markers_past_capacity() {
    let mut svc = MarkerFixture::service(2);
    let first = svc.place("A", [0, 0, 0], MarkerTarget::Structure(StructureId(1))).unwrap();
    svc.place("B", [0, 0, 0], MarkerTarget::Structure(StructureId(1))).unwrap();
    let result = svc.place("C", [0, 0, 0], MarkerTarget::Structure(StructureId(1)));
    assert_eq!(result, Err(MarkerError::Full { capacity: 2 }));
    assert_eq!(svc.markers().len(), 2);
    assert_eq!(svc.markers()[0].id, first);
}

#[test]
fn ids_are_not_reused_after_delete() {
    let mut svc = MarkerFixture::service(8);
    let a = svc.place("A", [0, 0, 0], MarkerTarget::Structure(StructureId(1))).unwrap();
    svc.delete(a).unwrap();
    let b = svc.place("B", [0, 0, 0], MarkerTarget::Structure(StructureId(1))).unwrap();
    assert_ne!(a, b);
}

#[test]
fn names_are_checked() {
    let mut svc = MarkerFixture::service(8);
    let target = MarkerTarget::Structure(StructureId(1));
    assert_eq!(svc.place("   ", [0, 0, 0], target), Err(MarkerError::NameEmpty));
    assert_eq!(svc.place(&"x".repeat(65), [0, 0, 0], target), Err(MarkerError::NameTooLong));
    let id = svc.place(" Home ", [0, 0, 0], target).unwrap();
    assert_eq!(svc.markers()[0].name, "Home");
    assert_eq!(svc.rename(id, ""), Err(MarkerError::NameEmpty));
    svc.rename(id, "Base").unwrap();
    assert_eq!(svc.markers()[0].name, "Base");
}

#[test]
fn marker_with_deleted_target_reports_missing() {
    let mut library = LibraryService::new(Library::new(), LibraryConfig::default());
    let structure = library.add_structure(NewStructureFixture::named("Tower")).unwrap();
    let project = library.create_project("Castle").unwrap();
    let mut svc = MarkerFixture::service(8);
    let on_structure = svc.place("S", [0, 0, 0], MarkerTarget::Structure(structure)).unwrap();
    let on_project = svc.place("P", [0, 0, 0], MarkerTarget::Project(project)).unwrap();
    assert_eq!(svc.target_state(on_structure, library.library()), TargetState::Ok);
    assert_eq!(svc.target_state(on_project, library.library()), TargetState::Ok);

    library.delete_structure(structure).unwrap();
    library.delete_project(project).unwrap();
    assert_eq!(svc.target_state(on_structure, library.library()), TargetState::Missing);
    assert_eq!(svc.target_state(on_project, library.library()), TargetState::Missing);
}

#[test]
fn assign_changes_the_target() {
    let mut svc = MarkerFixture::service(8);
    let id = svc.place("A", [0, 0, 0], MarkerTarget::Structure(StructureId(1))).unwrap();
    svc.assign(id, MarkerTarget::Project(ProjectId(7))).unwrap();
    assert_eq!(svc.markers()[0].target, MarkerTarget::Project(ProjectId(7)));
}

#[test]
fn uses_of_structure_and_project() {
    let mut svc = MarkerFixture::service(8);
    let a = svc.place("A", [0, 0, 0], MarkerTarget::Structure(StructureId(1))).unwrap();
    let b = svc.place("B", [0, 0, 0], MarkerTarget::Structure(StructureId(1))).unwrap();
    let c = svc.place("C", [0, 0, 0], MarkerTarget::Structure(StructureId(2))).unwrap();
    let d = svc.place("D", [0, 0, 0], MarkerTarget::Project(ProjectId(1))).unwrap();
    assert_eq!(svc.uses_of_structure(StructureId(1)), vec![a, b]);
    assert_eq!(svc.uses_of_structure(StructureId(2)), vec![c]);
    assert_eq!(svc.uses_of_project(ProjectId(1)), vec![d]);
    assert!(svc.uses_of_project(ProjectId(2)).is_empty());
}

#[test]
fn deactivate_marks_dirty_only_when_something_was_active() {
    let mut svc = MarkerFixture::service(8);
    let a = svc.place("A", [0, 0, 0], MarkerTarget::Structure(StructureId(1))).unwrap();
    svc.mark_saved();
    svc.deactivate();
    assert!(!svc.is_dirty());
    svc.activate(a).unwrap();
    svc.mark_saved();
    svc.deactivate();
    assert!(svc.is_dirty());
}
