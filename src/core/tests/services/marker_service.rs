use stratum_core::config::{LibraryConfig, MarkerConfig};
use stratum_core::errors::MarkerError;
use stratum_core::services::{LibraryService, MarkerService, MarkerStore};
use stratum_core::structs::action::HudAction;
use stratum_core::structs::library::{Library, ProjectId, ProjectKind, ProjectOrder, StructureId};
use stratum_core::structs::marker::{Marker, MarkerId, MarkerTarget, TargetState, ViewMode};

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
    library.set_project_kind(project, ProjectKind::BigBuild).unwrap();
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
    svc.assign(id, MarkerTarget::Project(ProjectId(7)), &Library::new(), &ProjectOrder::new()).unwrap();
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

// A library with the project "Bonsai Tree". "bonsai-leaves-bark" is imported first and
// "bonsai-bark-only" second, so by name the bark comes first.
struct Bonsai {
    library: LibraryService,
    markers: MarkerService,
    project: ProjectId,
    leaves: StructureId,
    bark: StructureId,
}

impl Bonsai {
    fn new() -> Self {
        let mut library = LibraryService::new(Library::new(), LibraryConfig::default());
        let project = library.create_project("Bonsai Tree").unwrap();
        let leaves = library.add_structure(NewStructureFixture::named("bonsai-leaves-bark")).unwrap();
        let bark = library.add_structure(NewStructureFixture::named("bonsai-bark-only")).unwrap();
        library.move_structure(leaves, Some(project)).unwrap();
        library.move_structure(bark, Some(project)).unwrap();
        Self { library, markers: MarkerFixture::service(8), project, leaves, bark }
    }

    fn place(&mut self, target: MarkerTarget) -> MarkerId {
        self.markers.place("m", [0, 0, 0], target).unwrap()
    }

    // A marker that cycles and shows `structure`.
    fn cycling(&mut self, structure: StructureId) -> MarkerId {
        let id = self.place(MarkerTarget::Structure(structure));
        self.reconcile();
        id
    }

    fn reconcile(&mut self) -> bool {
        self.markers.reconcile(self.library.library(), self.library.order())
    }

    fn assign(&mut self, id: MarkerId, target: MarkerTarget) {
        self.markers.assign(id, target, self.library.library(), self.library.order()).unwrap();
    }

    // Shows layer `layer` (of a three layer build) on the marker through the HUD.
    fn show_layer(&mut self, id: MarkerId, layer: u32) {
        self.markers.activate(id).unwrap();
        for _ in 0..=layer {
            self.markers.apply_hud(HudAction::LayerUp, 3, &[]).unwrap();
        }
    }

    fn marker(&self, id: MarkerId) -> &Marker {
        self.markers.markers().iter().find(|marker| marker.id == id).unwrap()
    }
}

// Writes a marker file in the shape from before `current`, with a `selected` index.
struct OldFile;

impl OldFile {
    fn markers(project: ProjectId, selected: u32) -> MarkerService {
        let mut seed = MarkerFixture::service(8);
        seed.place("old", [0, 0, 0], MarkerTarget::Project(project)).unwrap();
        let store = MarkerStore::new(std::path::PathBuf::new());
        let (_, bytes) = store.encode(seed.set()).unwrap();
        let text = String::from_utf8(bytes).unwrap();
        let mut old = String::new();
        for line in text.lines() {
            let key = line.trim_start();
            if key.starts_with("\"current\"") || key.starts_with("\"parts\"") {
                continue;
            }
            if key.starts_with("\"opacity_step\"") {
                old.push_str(&format!("\"selected\" = {selected}\n"));
            }
            old.push_str(line);
            old.push('\n');
        }
        let set = store.decode(&MarkerFixture::world(), old.as_bytes()).unwrap();
        MarkerService::new(set, MarkerConfig { max_per_world: 8 })
    }
}

#[test]
fn reconcile_turns_a_structure_marker_in_a_normal_project_into_a_cycling_marker() {
    let mut scene = Bonsai::new();
    let id = scene.place(MarkerTarget::Structure(scene.leaves));
    scene.markers.mark_saved();

    assert!(scene.reconcile());
    assert!(scene.markers.is_dirty());
    let marker = scene.marker(id);
    assert_eq!(marker.target, MarkerTarget::Project(scene.project));
    assert_eq!(marker.view.current, Some(scene.leaves));
    assert!(marker.view.parts.is_empty());
    assert!(!scene.reconcile());
}

#[test]
fn reconcile_reads_a_legacy_selected_index_in_library_order() {
    let mut scene = Bonsai::new();
    scene.markers = OldFile::markers(scene.project, 1);
    assert_eq!(scene.markers.markers()[0].view.current, None);

    assert!(scene.reconcile());
    assert_eq!(scene.markers.markers()[0].view.current, Some(scene.bark));

    scene.markers = OldFile::markers(scene.project, 0);
    scene.reconcile();
    assert_eq!(scene.markers.markers()[0].view.current, Some(scene.leaves));
}

#[test]
fn reconcile_opens_a_legacy_index_past_the_end_on_the_first_by_name() {
    let mut scene = Bonsai::new();
    scene.markers = OldFile::markers(scene.project, 9);
    assert!(scene.reconcile());
    assert_eq!(scene.markers.markers()[0].view.current, Some(scene.bark));
}

#[test]
fn reconcile_keeps_the_structure_when_it_moves_out_of_the_project() {
    let mut scene = Bonsai::new();
    let id = scene.cycling(scene.leaves);
    scene.library.move_structure(scene.leaves, None).unwrap();

    assert!(scene.reconcile());
    let marker = scene.marker(id);
    assert_eq!(marker.target, MarkerTarget::Structure(scene.leaves));
    assert_eq!(marker.view.current, None);
    assert_eq!(scene.markers.target_state(id, scene.library.library()), TargetState::Ok);
}

#[test]
fn reconcile_keeps_the_structure_when_the_project_is_deleted() {
    let mut scene = Bonsai::new();
    let id = scene.cycling(scene.leaves);
    scene.library.delete_project(scene.project).unwrap();

    assert!(scene.reconcile());
    assert_eq!(scene.marker(id).target, MarkerTarget::Structure(scene.leaves));
    assert_eq!(scene.markers.target_state(id, scene.library.library()), TargetState::Ok);
}

#[test]
fn reconcile_keeps_the_structure_when_the_project_becomes_a_big_build() {
    let mut scene = Bonsai::new();
    let id = scene.cycling(scene.bark);
    scene.library.set_project_kind(scene.project, ProjectKind::BigBuild).unwrap();

    assert!(scene.reconcile());
    let marker = scene.marker(id);
    assert_eq!(marker.target, MarkerTarget::Structure(scene.bark));
    assert_eq!(marker.view.current, None);
}

#[test]
fn reconcile_follows_the_structure_into_another_normal_project() {
    let mut scene = Bonsai::new();
    let id = scene.cycling(scene.leaves);
    let other = scene.library.create_project("Orchard").unwrap();
    let apple = scene.library.add_structure(NewStructureFixture::named("apple")).unwrap();
    scene.library.move_structure(apple, Some(other)).unwrap();
    scene.library.move_structure(scene.leaves, Some(other)).unwrap();

    assert!(scene.reconcile());
    let marker = scene.marker(id);
    assert_eq!(marker.target, MarkerTarget::Project(other));
    assert_eq!(marker.view.current, Some(scene.leaves));
    assert!(marker.view.parts.is_empty());
    assert!(!scene.reconcile());
}

#[test]
fn big_build_turned_normal_opens_on_the_first_by_name() {
    let mut scene = Bonsai::new();
    scene.library.set_project_kind(scene.project, ProjectKind::BigBuild).unwrap();
    scene.markers = OldFile::markers(scene.project, 0);
    scene.reconcile();
    scene.library.set_project_kind(scene.project, ProjectKind::Normal).unwrap();

    assert!(scene.reconcile());
    assert_eq!(scene.markers.markers()[0].view.current, Some(scene.bark));
}

#[test]
fn removing_the_current_structure_moves_to_the_next_by_name() {
    let mut scene = Bonsai::new();
    let id = scene.cycling(scene.bark);
    scene.assign(id, MarkerTarget::Structure(scene.leaves));
    scene.assign(id, MarkerTarget::Structure(scene.bark));
    assert_eq!(scene.marker(id).view.parts.len(), 1);

    scene.markers.structure_removing(scene.bark, scene.library.order());
    scene.library.delete_structure(scene.bark).unwrap();

    let marker = scene.marker(id);
    assert_eq!(marker.view.current, Some(scene.leaves));
    assert!(marker.view.parts.iter().all(|part| part.structure != scene.bark));
    assert!(!scene.reconcile());
    assert_eq!(scene.markers.target_state(id, scene.library.library()), TargetState::Ok);
}

#[test]
fn removing_the_last_by_name_wraps_to_the_first() {
    let mut scene = Bonsai::new();
    let id = scene.cycling(scene.leaves);

    scene.markers.structure_removing(scene.leaves, scene.library.order());
    scene.library.delete_structure(scene.leaves).unwrap();

    assert_eq!(scene.marker(id).view.current, Some(scene.bark));
}

#[test]
fn a_normal_project_with_no_structures_is_missing() {
    let mut scene = Bonsai::new();
    let empty = scene.library.create_project("Empty").unwrap();
    let id = scene.place(MarkerTarget::Project(empty));
    scene.reconcile();
    assert_eq!(scene.marker(id).view.current, None);
    assert_eq!(scene.markers.target_state(id, scene.library.library()), TargetState::Missing);

    let last = scene.cycling(scene.bark);
    scene.library.delete_structure(scene.bark).unwrap();
    assert_eq!(scene.markers.target_state(last, scene.library.library()), TargetState::Missing);
}

#[test]
fn assign_within_the_project_restores_the_saved_layer() {
    let mut scene = Bonsai::new();
    let id = scene.cycling(scene.leaves);
    scene.show_layer(id, 2);

    scene.assign(id, MarkerTarget::Structure(scene.bark));
    let marker = scene.marker(id);
    assert_eq!(marker.view.current, Some(scene.bark));
    assert_eq!((marker.view.layer, marker.view.view_mode), (0, ViewMode::Whole));

    scene.assign(id, MarkerTarget::Structure(scene.leaves));
    let marker = scene.marker(id);
    assert_eq!(marker.view.current, Some(scene.leaves));
    assert_eq!((marker.view.layer, marker.view.view_mode), (2, ViewMode::Layer));
}

#[test]
fn assign_to_the_own_project_keeps_the_current_structure() {
    let mut scene = Bonsai::new();
    let id = scene.cycling(scene.leaves);
    scene.show_layer(id, 1);
    let before = scene.marker(id).clone();

    scene.assign(id, MarkerTarget::Project(scene.project));
    assert_eq!(scene.marker(id), &before);
}

#[test]
fn assign_to_another_project_opens_its_first_structure_and_clears_parts() {
    let mut scene = Bonsai::new();
    let tower = scene.library.create_project("Tower").unwrap();
    let zed = scene.library.add_structure(NewStructureFixture::named("zed")).unwrap();
    let alpha = scene.library.add_structure(NewStructureFixture::named("Alpha")).unwrap();
    scene.library.move_structure(zed, Some(tower)).unwrap();
    scene.library.move_structure(alpha, Some(tower)).unwrap();
    let id = scene.cycling(scene.leaves);
    scene.assign(id, MarkerTarget::Structure(scene.bark));
    assert_eq!(scene.marker(id).view.parts.len(), 1);

    scene.assign(id, MarkerTarget::Project(tower));
    let marker = scene.marker(id);
    assert_eq!(marker.target, MarkerTarget::Project(tower));
    assert_eq!(marker.view.current, Some(alpha));
    assert!(marker.view.parts.is_empty());
}

#[test]
fn uses_of_structure_counts_a_cycling_marker_on_it() {
    let mut scene = Bonsai::new();
    let on_leaves = scene.cycling(scene.leaves);
    let on_bark = scene.cycling(scene.bark);
    let plain = scene.place(MarkerTarget::Structure(StructureId(99)));
    assert_eq!(scene.markers.uses_of_structure(scene.leaves), vec![on_leaves]);
    assert_eq!(scene.markers.uses_of_structure(scene.bark), vec![on_bark]);
    assert_eq!(scene.markers.uses_of_structure(StructureId(99)), vec![plain]);
    assert_eq!(scene.markers.uses_of_project(scene.project), vec![on_leaves, on_bark]);
}
