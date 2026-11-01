use std::time::Duration;

use stratum_core::config::StratumConfig;
use stratum_core::errors::{LibraryError, StructureError};
use stratum_core::services::{GhostService, LibraryStore, MarkerStore, MaterialsWorker, StratumController};
use stratum_core::structs::action::{HudAction, LibraryAction, MaterialScope};
use stratum_core::structs::frame::FrameStatus;
use stratum_core::structs::ghost::GhostReport;
use stratum_core::structs::hud::{HudControl, HudKey, HudMode};
use stratum_core::structs::import::ImportFailure;
use stratum_core::structs::layout::LayoutIssue;
use stratum_core::structs::library::{ProjectId, ProjectKind, StructureId};
use stratum_core::structs::marker::{Axis, MarkerTarget, Rotation, TargetState, WorldKey};
use stratum_core::structs::snapshot::ImportNotice;
use stratum_core::structs::structure::StructureSize;

use crate::support::{ControllerHarness, Frames, GhostScene, StructureFixture};

// Common steps: a controller in a world with one imported structure.
struct Scene;

impl Scene {
    fn stone() -> Vec<u8> {
        StructureFixture::full([2, 2, 2]).bytes()
    }

    // Imports `name` through the picker path and waits for it.
    fn import(controller: &mut StratumController, dir: &std::path::Path, name: &str, bytes: &[u8]) {
        let path = ControllerHarness::picked_file(dir, &format!("{name}.mcstructure"), bytes);
        controller.files_picked(vec![path]);
        ControllerHarness::settle(controller, &Frames::main_menu(0.0));
    }

    fn marker_target(controller: &StratumController) -> MarkerTarget {
        MarkerTarget::Structure(controller.snapshot().unsorted[0].id)
    }

    fn place(controller: &mut StratumController, name: &str) {
        let target = Self::marker_target(controller);
        controller.apply(LibraryAction::PlaceMarker { name: name.to_string(), target });
    }

    fn notices(controller: &StratumController) -> Vec<ImportNotice> {
        controller.snapshot().notices.clone()
    }
}

#[test]
fn world_change_switches_marker_set() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    Scene::import(&mut controller, data.path(), "tower", &Scene::stone());

    controller.tick(&Frames::in_world("W1", 1.0));
    Scene::place(&mut controller, "Home");
    assert_eq!(controller.snapshot().markers.len(), 1);

    controller.tick(&Frames::in_world("W2", 2.0));
    assert!(controller.snapshot().markers.is_empty());

    controller.tick(&Frames::in_world("W1", 3.0));
    assert_eq!(controller.snapshot().markers[0].name, "Home");

    controller.tick(&Frames::in_dimension("W1", -1, 4.0));
    assert!(controller.snapshot().markers.is_empty());

    controller.tick(&Frames::in_world("W2", 5.0));
    controller.shutdown();
    let store = MarkerStore::new(data.path().join("markers"));
    let w1 = store.load(&WorldKey { world: "W1".to_string(), dimension: 0 }).unwrap();
    let w2 = store.load(&WorldKey { world: "W2".to_string(), dimension: 0 }).unwrap();
    assert_eq!(w1.markers.len(), 1);
    assert!(w2.markers.is_empty());
}

#[test]
fn leaving_the_world_unloads_markers() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    Scene::import(&mut controller, data.path(), "tower", &Scene::stone());
    controller.tick(&Frames::in_world("W1", 1.0));
    Scene::place(&mut controller, "Home");

    controller.tick(&Frames::main_menu(2.0));
    let snapshot = controller.snapshot();
    assert!(!snapshot.in_world);
    assert!(snapshot.markers.is_empty());
    // The marker was queued for saving on the way out, before the throttle allowed a save.
    ControllerHarness::flush(&controller);
    let saved = MarkerStore::new(data.path().join("markers"))
        .load(&WorldKey { world: "W1".to_string(), dimension: 0 })
        .unwrap();
    assert_eq!(saved.markers.len(), 1);

    controller.tick(&Frames::in_world("W1", 3.0));
    assert_eq!(controller.snapshot().markers.len(), 1);
}

#[test]
fn place_marker_uses_player_feet() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    Scene::import(&mut controller, data.path(), "tower", &Scene::stone());
    controller.tick(&Frames::at("W1", [10, 70, -20], 1.0));
    Scene::place(&mut controller, "Home");
    assert_eq!(controller.snapshot().markers[0].origin, [10, 70, -20]);

    // the row gives the stored position, not one measured from the feet
    controller.tick(&Frames::at("W1", [13, 70, -16], 2.0));
    assert_eq!(controller.snapshot().markers[0].origin, [10, 70, -20]);

    controller.shutdown();
    let saved = MarkerStore::new(data.path().join("markers"))
        .load(&WorldKey { world: "W1".to_string(), dimension: 0 })
        .unwrap();
    assert_eq!(saved.markers[0].origin, [10, 70, -20]);
}

#[test]
fn place_marker_outside_a_world_does_nothing() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    Scene::import(&mut controller, data.path(), "tower", &Scene::stone());
    controller.tick(&Frames::main_menu(1.0));
    Scene::place(&mut controller, "Home");
    assert!(controller.snapshot().markers.is_empty());
    assert_eq!(Scene::notices(&controller), vec![ImportNotice::NotInWorld]);

    // A world with no player yet does the same.
    let no_player = FrameStatus { has_player: false, ..Frames::in_world("W1", 2.0) };
    controller.tick(&no_player);
    Scene::place(&mut controller, "Home");
    assert!(controller.snapshot().markers.is_empty());

    controller.shutdown();
    assert!(
        MarkerStore::new(data.path().join("markers"))
            .load(&WorldKey { world: "W1".to_string(), dimension: 0 })
            .unwrap()
            .markers
            .is_empty()
    );
}

#[test]
fn a_world_with_no_key_yet_counts_as_no_world() {
    let data = tempfile::tempdir().unwrap();
    let markers_dir = data.path().join("markers");
    let mut controller = ControllerHarness::start(data.path());
    Scene::import(&mut controller, data.path(), "tower", &Scene::stone());
    controller.tick(&Frames::in_world("W1", 1.0));
    Scene::place(&mut controller, "Home");
    controller.tick(&Frames::main_menu(2.0));
    ControllerHarness::flush(&controller);
    let saved_files = ControllerHarness::file_names(&markers_dir);
    assert_eq!(saved_files.len(), 1);

    // In a world whose key is still empty: no marker set, and placing is refused.
    controller.tick(&Frames::in_world("", 3.0));
    assert!(!controller.snapshot().in_world);
    assert!(controller.snapshot().markers.is_empty());
    Scene::place(&mut controller, "Away");
    assert!(controller.snapshot().markers.is_empty());
    assert_eq!(Scene::notices(&controller), vec![ImportNotice::NotInWorld]);
    controller.tick(&Frames::in_world("", 5.0));
    ControllerHarness::flush(&controller);
    assert_eq!(ControllerHarness::file_names(&markers_dir), saved_files);

    // Once the key arrives, the world's markers load.
    controller.tick(&Frames::in_world("W1", 6.0));
    assert!(controller.snapshot().in_world);
    assert_eq!(controller.snapshot().markers.len(), 1);
    assert_eq!(controller.snapshot().markers[0].name, "Home");
}

#[test]
fn the_snapshot_follows_whether_the_game_has_a_player() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    controller.tick(&Frames::in_world("W1", 1.0));
    assert!(controller.snapshot().has_player);
    controller.tick(&FrameStatus { has_player: false, ..Frames::in_world("W1", 2.0) });
    assert!(controller.snapshot().in_world);
    assert!(!controller.snapshot().has_player);
    controller.tick(&Frames::in_world("W1", 3.0));
    assert!(controller.snapshot().has_player);
}

#[test]
fn placing_a_marker_makes_it_the_active_one() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    Scene::import(&mut controller, data.path(), "tower", &Scene::stone());
    controller.tick(&Frames::in_world("W1", 1.0));
    Scene::place(&mut controller, "First");
    controller.tick(&Frames::in_world("W1", 2.0));
    Scene::place(&mut controller, "Second");

    let rows = controller.snapshot().markers.clone();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows.iter().filter(|row| row.active).count(), 1);
    assert!(rows.iter().find(|row| row.name == "Second").unwrap().active);
    assert!(!rows.iter().find(|row| row.name == "First").unwrap().active);
    assert_eq!(rows[0].state, TargetState::Ok);
    assert_eq!(rows[0].target_name, "tower");

    let first = rows.iter().find(|row| row.name == "First").unwrap().id;
    controller.apply(LibraryAction::ActivateMarker(first));
    assert!(controller.snapshot().markers.iter().find(|row| row.name == "First").unwrap().active);
    controller.apply(LibraryAction::DeactivateMarker);
    assert!(controller.snapshot().markers.iter().all(|row| !row.active));
}

#[test]
fn a_full_marker_set_refuses_with_a_notice() {
    let data = tempfile::tempdir().unwrap();
    let mut config = StratumConfig::default();
    config.markers.max_per_world = 1;
    let mut controller = StratumController::new(data.path().to_path_buf(), config);
    Scene::import(&mut controller, data.path(), "tower", &Scene::stone());
    controller.tick(&Frames::in_world("W1", 1.0));
    Scene::place(&mut controller, "First");
    controller.tick(&Frames::in_world("W1", 2.0));
    Scene::place(&mut controller, "Second");
    assert_eq!(controller.snapshot().markers.len(), 1);
    assert!(matches!(Scene::notices(&controller)[0], ImportNotice::MarkerRefused(_)));
}

#[test]
fn imports_arrive_in_the_snapshot() {
    let data = tempfile::tempdir().unwrap();
    let files = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    assert_eq!(controller.snapshot().revision, 0);

    let good = ControllerHarness::picked_file(files.path(), "tower.mcstructure", &Scene::stone());
    let bad = ControllerHarness::picked_file(files.path(), "junk.mcstructure", b"not a structure");
    controller.files_picked(vec![good.clone(), bad.clone()]);
    ControllerHarness::settle(&mut controller, &Frames::main_menu(1.0));

    let snapshot = controller.snapshot();
    assert_eq!(snapshot.unsorted.len(), 1);
    assert_eq!(snapshot.unsorted[0].name, "tower");
    assert_eq!(snapshot.unsorted[0].size, StructureSize::new(2, 2, 2));
    assert_eq!(snapshot.unsorted[0].block_count, 8);
    assert_eq!(snapshot.all_count, 1);
    assert!(snapshot.revision > 0);
    assert!(matches!(
        snapshot.notices.as_slice(),
        [ImportNotice::ImportFailed { file, reason: ImportFailure::NotStructure(_) }] if file == "junk.mcstructure"
    ));
    // Picker imports never remove their source.
    assert!(good.exists());
    assert!(bad.exists());
    assert_eq!(ControllerHarness::file_names(&data.path().join("structures")), ["tower.mcstructure"]);
}

#[test]
fn the_action_import_files_works_like_the_picker() {
    let data = tempfile::tempdir().unwrap();
    let files = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    let path = ControllerHarness::picked_file(files.path(), "a.mcstructure", &Scene::stone());
    controller.apply(LibraryAction::ImportFiles(vec![path.clone()]));
    ControllerHarness::settle(&mut controller, &Frames::main_menu(1.0));
    assert_eq!(controller.snapshot().unsorted.len(), 1);
    assert!(path.exists());
}

#[test]
fn the_file_picker_request_is_taken_once() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    assert!(!controller.take_picker_request());
    controller.apply(LibraryAction::RequestFilePicker(None));
    assert!(controller.snapshot().picker_pending);
    assert!(controller.take_picker_request());
    assert!(!controller.take_picker_request());

    // The picker stays open until its answer: the flag holds, and a second request is not handed out.
    assert!(controller.snapshot().picker_pending);
    controller.apply(LibraryAction::RequestFilePicker(None));
    assert!(!controller.take_picker_request());
    controller.files_picked(Vec::new());
    assert!(!controller.snapshot().picker_pending);
    assert!(!controller.take_picker_request());
}

#[test]
fn inbox_files_are_imported_and_removed() {
    let data = tempfile::tempdir().unwrap();
    let source = ControllerHarness::inbox_file(data.path(), "castle.mcstructure", &Scene::stone());
    let mut controller = ControllerHarness::start(data.path());

    // The first scan only records the size; the second lists the file.
    controller.tick(&Frames::main_menu(0.0));
    assert!(controller.snapshot().unsorted.is_empty());
    controller.tick(&Frames::main_menu(2.0));
    ControllerHarness::settle(&mut controller, &Frames::main_menu(2.0));
    assert_eq!(ControllerHarness::unsorted_names(&controller), ["castle"]);

    // The source is removed on the first tick after the library file reached the disk.
    ControllerHarness::flush(&controller);
    controller.tick(&Frames::main_menu(2.0));
    assert!(!source.exists());
    assert_eq!(ControllerHarness::file_names(&data.path().join("structures")), ["castle.mcstructure"]);
}

#[test]
fn the_inbox_is_polled_on_its_interval() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    controller.tick(&Frames::main_menu(0.0));
    ControllerHarness::inbox_file(data.path(), "castle.mcstructure", &Scene::stone());
    // Scans are 2 seconds apart, so these two ticks do not scan and nothing is listed yet.
    controller.tick(&Frames::main_menu(0.5));
    controller.tick(&Frames::main_menu(1.0));
    assert_eq!(controller.pending_imports(), 0);
    controller.tick(&Frames::main_menu(2.0));
    controller.tick(&Frames::main_menu(2.5));
    assert_eq!(controller.pending_imports(), 0);
    controller.tick(&Frames::main_menu(4.0));
    assert_eq!(controller.pending_imports(), 1);
    ControllerHarness::settle(&mut controller, &Frames::main_menu(4.0));
    assert_eq!(controller.snapshot().unsorted.len(), 1);
}

#[test]
fn a_refused_inbox_file_is_kept_and_its_stored_copy_removed() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = StratumController::new(data.path().to_path_buf(), ControllerHarness::config(1, 64));
    let first = ControllerHarness::inbox_file(data.path(), "a.mcstructure", &Scene::stone());
    controller.tick(&Frames::main_menu(0.0));
    controller.tick(&Frames::main_menu(2.0));
    ControllerHarness::settle(&mut controller, &Frames::main_menu(2.0));
    ControllerHarness::flush(&controller);
    controller.tick(&Frames::main_menu(2.0));
    assert!(!first.exists());

    let second = ControllerHarness::inbox_file(data.path(), "b.mcstructure", &Scene::stone());
    let mut time = 4.0;
    controller.tick(&Frames::main_menu(time));
    time += 2.0;
    controller.tick(&Frames::main_menu(time));
    ControllerHarness::settle(&mut controller, &Frames::main_menu(time));

    assert!(second.exists());
    assert_eq!(controller.snapshot().unsorted.len(), 1);
    assert_eq!(ControllerHarness::file_names(&data.path().join("structures")), ["a.mcstructure"]);
    assert!(Scene::notices(&controller).contains(&ImportNotice::ImportRefused {
        file: "b.mcstructure".to_string(),
        error: LibraryError::Full { capacity: 1 },
    }));

    // The kept file is not submitted again this session.
    for _ in 0..5 {
        time += 2.0;
        controller.tick(&Frames::main_menu(time));
        assert_eq!(controller.pending_imports(), 0);
    }
    assert!(second.exists());
}

#[test]
fn a_failed_inbox_file_is_set_aside() {
    let data = tempfile::tempdir().unwrap();
    let junk = ControllerHarness::inbox_file(data.path(), "junk.mcstructure", b"not a structure");
    let mut controller = ControllerHarness::start(data.path());
    controller.tick(&Frames::main_menu(0.0));
    controller.tick(&Frames::main_menu(2.0));
    ControllerHarness::settle(&mut controller, &Frames::main_menu(2.0));

    assert!(!junk.exists());
    assert_eq!(ControllerHarness::file_names(&data.path().join("import")), ["junk.mcstructure.failed"]);
    assert!(controller.snapshot().unsorted.is_empty());
    assert!(matches!(Scene::notices(&controller)[0], ImportNotice::ImportFailed { .. }));
}

#[test]
fn inbox_files_met_by_a_full_queue_are_listed_again() {
    let data = tempfile::tempdir().unwrap();
    for number in 0..10 {
        ControllerHarness::inbox_file(data.path(), &format!("s{number}.mcstructure"), &Scene::stone());
    }
    let mut controller = StratumController::new(data.path().to_path_buf(), ControllerHarness::config(100, 1));
    controller.tick(&Frames::main_menu(0.0));
    // The queue holds one request and the result channel one result, and nothing is drained
    // during this tick, so most of the ten files are refused.
    controller.tick(&Frames::main_menu(2.0));
    assert!(controller.pending_imports() <= 2 + 1);

    ControllerHarness::tick_until_unsorted(&mut controller, "W1", 4.0, 10);
    ControllerHarness::settle(&mut controller, &Frames::main_menu(1000.0));
    ControllerHarness::flush(&controller);
    controller.tick(&Frames::main_menu(1000.0));
    assert_eq!(controller.snapshot().unsorted.len(), 10);
    assert!(ControllerHarness::file_names(&data.path().join("import")).is_empty());
}

#[test]
fn a_read_only_library_imports_nothing() {
    let data = tempfile::tempdir().unwrap();
    let files = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(data.path().join("library.hcl")).unwrap();
    let source = ControllerHarness::inbox_file(data.path(), "castle.mcstructure", &Scene::stone());
    let mut controller = ControllerHarness::start(data.path());

    controller.tick(&Frames::main_menu(0.0));
    controller.tick(&Frames::main_menu(2.0));
    controller.tick(&Frames::main_menu(4.0));
    assert_eq!(controller.pending_imports(), 0);

    let picked = ControllerHarness::picked_file(files.path(), "tower.mcstructure", &Scene::stone());
    controller.files_picked(vec![picked]);
    assert_eq!(controller.pending_imports(), 0);
    ControllerHarness::settle(&mut controller, &Frames::main_menu(5.0));

    assert!(controller.snapshot().unsorted.is_empty());
    assert!(source.exists());
    assert!(ControllerHarness::file_names(&data.path().join("structures")).is_empty());
    let unavailable = Scene::notices(&controller).iter().filter(|n| **n == ImportNotice::LibraryUnavailable).count();
    assert_eq!(unavailable, 2);
}

#[test]
fn an_inbox_source_stays_until_the_library_is_on_disk() {
    let data = tempfile::tempdir().unwrap();
    let source = ControllerHarness::inbox_file(data.path(), "castle.mcstructure", &Scene::stone());
    // A folder in the way of the temporary file makes every library write fail.
    let blocker = data.path().join("library.hcl.tmp");
    std::fs::create_dir_all(&blocker).unwrap();
    let mut controller = ControllerHarness::start(data.path());
    controller.tick(&Frames::main_menu(0.0));
    controller.tick(&Frames::main_menu(2.0));
    ControllerHarness::settle(&mut controller, &Frames::main_menu(2.0));
    assert_eq!(ControllerHarness::unsorted_names(&controller), ["castle"]);
    ControllerHarness::wait_for_failures(&controller, 1);
    controller.tick(&Frames::main_menu(2.1));
    assert!(source.exists());

    // The game closes while the library cannot be written: the source is still there.
    controller.shutdown();
    drop(controller);
    assert!(source.exists());

    // The next start imports it again, and removes it once the library reached the disk.
    std::fs::remove_dir_all(&blocker).unwrap();
    let mut next = ControllerHarness::start(data.path());
    ControllerHarness::tick_until_unsorted(&mut next, "W1", 10.0, 1);
    ControllerHarness::tick_until_gone(&mut next, &source, 100.0);
    assert_eq!(ControllerHarness::unsorted_names(&next), ["castle"]);
    assert_eq!(ControllerHarness::file_names(&data.path().join("structures")), ["castle.mcstructure"]);
    assert_eq!(LibraryStore::new(data.path().to_path_buf()).load().unwrap().structures.len(), 1);
}

#[test]
fn orphans_in_the_store_are_removed_at_start() {
    let data = tempfile::tempdir().unwrap();
    let mut seed = ControllerHarness::start(data.path());
    Scene::import(&mut seed, data.path(), "kept", &Scene::stone());
    seed.shutdown();
    let structures = data.path().join("structures");
    std::fs::write(structures.join("orphan.mcstructure"), b"left by a shutdown").unwrap();
    assert_eq!(ControllerHarness::file_names(&structures), ["kept.mcstructure", "orphan.mcstructure"]);

    let controller = ControllerHarness::start(data.path());
    assert_eq!(ControllerHarness::file_names(&structures), ["kept.mcstructure"]);
    assert_eq!(controller.snapshot().unsorted.len(), 1);
}

#[test]
fn a_corrupt_library_file_does_not_cost_the_stored_structures() {
    let data = tempfile::tempdir().unwrap();
    let mut seed = ControllerHarness::start(data.path());
    Scene::import(&mut seed, data.path(), "kept", &Scene::stone());
    seed.shutdown();
    std::fs::write(data.path().join("library.hcl"), b"{{ not hcl").unwrap();

    let controller = ControllerHarness::start(data.path());
    assert!(controller.snapshot().unsorted.is_empty());
    assert_eq!(ControllerHarness::file_names(&data.path().join("structures")), ["kept.mcstructure"]);
}

#[test]
fn deleting_a_structure_removes_its_file_and_cache_entry() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    Scene::import(&mut controller, data.path(), "tower", &Scene::stone());
    let id = controller.snapshot().unsorted[0].id;
    assert_eq!(controller.cache_resident_bytes(), 0);

    controller.apply(LibraryAction::LoadMaterials(MaterialScope::Structure(id)));
    ControllerHarness::tick_until_materials_ready(&mut controller, &Frames::main_menu(1.0));
    assert!(controller.cache_resident_bytes() > 0);

    controller.apply(LibraryAction::DeleteStructure(id));
    assert_eq!(controller.cache_resident_bytes(), 0);
    assert!(ControllerHarness::file_names(&data.path().join("structures")).is_empty());
    assert!(controller.snapshot().unsorted.is_empty());
    assert_eq!(controller.snapshot().all_count, 0);
    // The materials of a deleted structure are dropped.
    assert!(controller.snapshot().materials.is_none());
}

#[test]
fn a_marker_of_a_deleted_structure_shows_missing() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    Scene::import(&mut controller, data.path(), "tower", &Scene::stone());
    controller.tick(&Frames::in_world("W1", 1.0));
    Scene::place(&mut controller, "Home");
    let id = controller.snapshot().unsorted[0].id;
    controller.apply(LibraryAction::DeleteStructure(id));
    let row = controller.snapshot().markers[0].clone();
    assert_eq!(row.state, TargetState::Missing);
    assert_eq!(row.target_name, "");
}

#[test]
fn project_materials_merge_structures() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    let a = StructureFixture::new([2, 1, 1]).block(0, 0, 0, "minecraft:stone").block(1, 0, 0, "minecraft:oak_planks");
    let b = StructureFixture::full([1, 1, 3]);
    Scene::import(&mut controller, data.path(), "a", &a.bytes());
    Scene::import(&mut controller, data.path(), "b", &b.bytes());
    assert!(controller.snapshot().materials.is_none());

    controller.apply(LibraryAction::CreateProject("Castle".to_string()));
    let project = controller.snapshot().projects[0].id;
    for row in controller.snapshot().unsorted.clone() {
        controller.apply(LibraryAction::MoveStructure(row.id, Some(project)));
    }
    assert!(controller.snapshot().unsorted.is_empty());
    assert_eq!(controller.snapshot().projects[0].structures.len(), 2);
    // Nothing is counted until asked.
    assert!(controller.snapshot().materials.is_none());

    controller.apply(LibraryAction::LoadMaterials(MaterialScope::Project(project)));
    ControllerHarness::tick_until_materials_ready(&mut controller, &Frames::main_menu(1.0));
    let snapshot = controller.snapshot();
    let (scope, list) = snapshot.materials.as_ref().unwrap();
    assert_eq!(*scope, MaterialScope::Project(project));
    let counts: Vec<(String, u64)> =
        list.lines().iter().map(|line| (line.item.short_name().to_string(), line.count)).collect();
    assert_eq!(counts, [("stone".to_string(), 4), ("oak_planks".to_string(), 1)]);

    let a_id = snapshot.projects[0].structures.iter().find(|row| row.name == "a").unwrap().id;
    // Both structures are cached now, so this one is counted at once.
    controller.apply(LibraryAction::LoadMaterials(MaterialScope::Structure(a_id)));
    assert_eq!(controller.snapshot().materials_loading, None);
    let snapshot = controller.snapshot();
    let (scope, list) = snapshot.materials.as_ref().unwrap();
    assert_eq!(*scope, MaterialScope::Structure(a_id));
    assert_eq!(list.total(), 2);
}

#[test]
fn loading_materials_of_a_broken_file_gives_a_notice() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    Scene::import(&mut controller, data.path(), "tower", &Scene::stone());
    let id = controller.snapshot().unsorted[0].id;
    std::fs::remove_file(data.path().join("structures").join("tower.mcstructure")).unwrap();
    controller.apply(LibraryAction::LoadMaterials(MaterialScope::Structure(id)));
    ControllerHarness::tick_until_materials_ready(&mut controller, &Frames::main_menu(1.0));
    assert!(controller.snapshot().materials.is_none());
    assert_eq!(controller.snapshot().materials_failed, Some(MaterialScope::Structure(id)));
    assert!(matches!(Scene::notices(&controller)[..], [ImportNotice::MaterialsFailed(StructureError::Unreadable(_))]));
}

#[test]
fn a_new_load_clears_the_failure_and_a_list_keeps_it_clear() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    Scene::import(&mut controller, data.path(), "tower", &Scene::stone());
    let broken = controller.snapshot().unsorted[0].id;
    std::fs::remove_file(data.path().join("structures").join("tower.mcstructure")).unwrap();
    controller.apply(LibraryAction::LoadMaterials(MaterialScope::Structure(broken)));
    ControllerHarness::tick_until_materials_ready(&mut controller, &Frames::main_menu(1.0));
    assert_eq!(controller.snapshot().materials_failed, Some(MaterialScope::Structure(broken)));

    Scene::import(&mut controller, data.path(), "hall", &Scene::stone());
    let good = controller.snapshot().unsorted.iter().find(|row| row.name == "hall").unwrap().id;
    let scope = MaterialScope::Structure(good);
    controller.apply(LibraryAction::LoadMaterials(scope));
    let started = controller.snapshot();
    assert_eq!(started.materials_loading, Some(scope));
    assert_eq!(started.materials_failed, None);
    ControllerHarness::tick_until_materials_ready(&mut controller, &Frames::main_menu(2.0));
    let ready = controller.snapshot();
    assert!(ready.materials.is_some());
    assert_eq!(ready.materials_failed, None);
}

#[test]
fn a_load_of_an_unknown_scope_is_a_failure() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    let scope = MaterialScope::Structure(stratum_core::structs::library::StructureId(9999));
    controller.apply(LibraryAction::LoadMaterials(scope));
    assert_eq!(controller.snapshot().materials_failed, Some(scope));
    assert!(controller.snapshot().materials.is_none());
}

#[test]
fn big_build_projects_get_a_layout() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    controller.apply(LibraryAction::CreateProject("Keep".to_string()));
    let project = controller.snapshot().projects[0].id;
    assert!(controller.snapshot().layouts.is_empty());

    controller.apply(LibraryAction::SetProjectKind(project, ProjectKind::BigBuild));
    assert_eq!(controller.snapshot().layouts, vec![(project, Err(LayoutIssue::NoPieces))]);

    let piece = StructureFixture::full([2, 2, 2]).origin([10, 60, 10]);
    Scene::import(&mut controller, data.path(), "piece", &piece.bytes());
    let id = controller.snapshot().unsorted[0].id;
    controller.apply(LibraryAction::MoveStructure(id, Some(project)));
    let snapshot = controller.snapshot();
    let (_, layout) = &snapshot.layouts[0];
    assert_eq!(layout.as_ref().unwrap().size, StructureSize::new(2, 2, 2));
}

#[test]
fn library_actions_that_the_library_refuses_give_a_notice() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    controller.apply(LibraryAction::CreateProject("   ".to_string()));
    assert_eq!(Scene::notices(&controller), vec![ImportNotice::LibraryRefused(LibraryError::NameEmpty)]);
    assert!(controller.snapshot().projects.is_empty());
}

#[test]
fn snapshot_revision_only_moves_on_change() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    controller.tick(&Frames::in_world("W1", 0.0));
    let first = controller.snapshot();
    let same = controller.snapshot();
    assert!(std::sync::Arc::ptr_eq(&first, &same));

    controller.tick(&Frames::in_world("W1", 0.1));
    controller.tick(&Frames::in_world("W1", 0.2));
    assert_eq!(controller.snapshot().revision, first.revision);

    controller.apply(LibraryAction::CreateProject("Castle".to_string()));
    let after_create = controller.snapshot().revision;
    assert!(after_create > first.revision);

    controller.tick(&Frames::in_world("W1", 0.3));
    assert_eq!(controller.snapshot().revision, after_create);

    // Leaving the world changes the snapshot.
    controller.tick(&Frames::main_menu(0.4));
    assert!(controller.snapshot().revision > after_create);
}

#[test]
fn walking_leaves_the_marker_rows_and_the_revision_alone() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    Scene::import(&mut controller, data.path(), "tower", &Scene::stone());
    controller.tick(&Frames::at("W1", [-3, -12, -340], 1.0));
    Scene::place(&mut controller, "Home");
    controller.tick(&Frames::at("W1", [-3, -12, -340], 1.1));
    let placed = controller.snapshot().revision;
    assert_eq!(controller.snapshot().markers[0].origin, [-3, -12, -340]);

    controller.tick(&Frames::at("W1", [40, 64, 3], 1.2));
    controller.tick(&Frames::at("W1", [-90, 70, 12], 1.3));
    assert_eq!(controller.snapshot().markers[0].origin, [-3, -12, -340]);
    assert_eq!(controller.snapshot().revision, placed);
}

#[test]
fn a_library_change_rebuilds_the_rows() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    Scene::import(&mut controller, data.path(), "tower", &Scene::stone());
    controller.tick(&Frames::at("W1", [0, 64, 0], 1.0));
    Scene::place(&mut controller, "Home");
    let before = controller.snapshot();

    let id = before.unsorted[0].id;
    controller.apply(LibraryAction::RenameStructure(id, "Keep".to_string()));
    let after = controller.snapshot();
    assert!(after.revision > before.revision);
    assert_eq!(after.unsorted[0].name, "Keep");
    // The marker row follows the new name of its target.
    assert_eq!(after.markers[0].target_name, "Keep");
}

#[test]
fn notices_are_bounded_and_drop_the_oldest() {
    let data = tempfile::tempdir().unwrap();
    let files = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    let missing: Vec<_> = (0..20).map(|n| files.path().join(format!("m{n}.mcstructure"))).collect();
    controller.files_picked(missing);
    ControllerHarness::settle(&mut controller, &Frames::main_menu(1.0));

    let notices = Scene::notices(&controller);
    assert_eq!(notices.len(), 16);
    let names: Vec<&str> = notices
        .iter()
        .filter_map(|notice| match notice {
            ImportNotice::ImportFailed { file, .. } => Some(file.as_str()),
            _ => None,
        })
        .collect();
    assert!(!names.contains(&"m0.mcstructure"));
    assert!(names.contains(&"m19.mcstructure"));
}

#[test]
fn notices_expire() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    controller.tick(&Frames::main_menu(100.0));
    controller.apply(LibraryAction::CreateProject(String::new()));
    assert_eq!(Scene::notices(&controller).len(), 1);
    controller.tick(&Frames::main_menu(105.0));
    assert_eq!(Scene::notices(&controller).len(), 1);
    controller.tick(&Frames::main_menu(111.0));
    assert!(Scene::notices(&controller).is_empty());
}

#[test]
fn dirty_state_is_saved_at_most_once_a_second() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    let store = LibraryStore::new(data.path().to_path_buf());

    controller.tick(&Frames::main_menu(10.0));
    controller.apply(LibraryAction::CreateProject("One".to_string()));
    controller.tick(&Frames::main_menu(10.0));
    ControllerHarness::flush(&controller);
    assert_eq!(store.load().unwrap().projects.len(), 1);

    controller.apply(LibraryAction::CreateProject("Two".to_string()));
    controller.tick(&Frames::main_menu(10.5));
    ControllerHarness::flush(&controller);
    assert_eq!(store.load().unwrap().projects.len(), 1);
    controller.tick(&Frames::main_menu(11.0));
    ControllerHarness::flush(&controller);
    assert_eq!(store.load().unwrap().projects.len(), 2);
}

#[test]
fn shutdown_saves_dirty_state() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    Scene::import(&mut controller, data.path(), "tower", &Scene::stone());
    controller.tick(&Frames::in_world("W1", 1.0));
    // No tick follows, so only shutdown can save these changes.
    controller.apply(LibraryAction::CreateProject("Castle".to_string()));
    Scene::place(&mut controller, "Home");
    controller.shutdown();

    let library = LibraryStore::new(data.path().to_path_buf()).load().unwrap();
    assert_eq!(library.projects.len(), 1);
    assert_eq!(library.structures.len(), 1);
    let markers = MarkerStore::new(data.path().join("markers"))
        .load(&WorldKey { world: "W1".to_string(), dimension: 0 })
        .unwrap();
    assert_eq!(markers.markers.len(), 1);
    assert!(markers.active.is_some());

    // A later start sees the saved state.
    let restarted = ControllerHarness::start(data.path());
    assert_eq!(restarted.snapshot().projects.len(), 1);
    assert_eq!(restarted.snapshot().unsorted.len(), 1);
}

#[test]
fn nothing_is_accepted_after_shutdown() {
    let data = tempfile::tempdir().unwrap();
    let files = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    controller.shutdown();
    controller.shutdown();
    let path = ControllerHarness::picked_file(files.path(), "a.mcstructure", &Scene::stone());
    controller.files_picked(vec![path]);
    assert_eq!(controller.pending_imports(), 0);
    assert!(matches!(Scene::notices(&controller)[..], [ImportNotice::ImportQueueFull]));
}

#[test]
fn a_material_list_that_needs_a_file_read_arrives_on_a_later_tick() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    Scene::import(&mut controller, data.path(), "tower", &Scene::stone());
    let id = controller.snapshot().unsorted[0].id;
    let scope = MaterialScope::Structure(id);

    // A cache miss returns at once with the loading state.
    controller.apply(LibraryAction::LoadMaterials(scope));
    let loading = controller.snapshot();
    assert_eq!(loading.materials_loading, Some(scope));
    assert!(loading.materials.is_none());

    ControllerHarness::tick_until_materials_ready(&mut controller, &Frames::main_menu(1.0));
    let ready = controller.snapshot();
    assert_eq!(ready.materials_loading, None);
    assert_eq!(ready.materials.as_ref().unwrap().1.total(), 8);
    assert!(ready.revision > loading.revision);

    // A cache hit is counted before apply returns.
    controller.apply(LibraryAction::LoadMaterials(scope));
    let hit = controller.snapshot();
    assert_eq!(hit.materials_loading, None);
    assert_eq!(hit.materials.as_ref().unwrap().1.total(), 8);
}

#[test]
fn deleting_a_structure_cancels_its_material_list() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    Scene::import(&mut controller, data.path(), "tower", &Scene::stone());
    let id = controller.snapshot().unsorted[0].id;
    controller.apply(LibraryAction::LoadMaterials(MaterialScope::Structure(id)));
    controller.apply(LibraryAction::DeleteStructure(id));
    assert_eq!(controller.snapshot().materials_loading, None);
    ControllerHarness::settle(&mut controller, &Frames::main_menu(1.0));
    controller.tick(&Frames::main_menu(2.0));
    assert!(controller.snapshot().materials.is_none());
    assert_eq!(controller.cache_resident_bytes(), 0);
}

#[test]
fn an_unreadable_library_file_is_read_only_for_the_session() {
    let data = tempfile::tempdir().unwrap();
    // A folder where the file should be cannot be read as a file.
    std::fs::create_dir_all(data.path().join("library.hcl")).unwrap();
    let mut controller = ControllerHarness::start(data.path());
    assert!(controller.snapshot().library_read_only);

    controller.tick(&Frames::main_menu(50.0));
    assert!(Scene::notices(&controller).contains(&ImportNotice::LibraryUnavailable));
    controller.tick(&Frames::main_menu(55.0));
    assert!(Scene::notices(&controller).contains(&ImportNotice::LibraryUnavailable));

    // The notice expires; the flag does not.
    controller.tick(&Frames::main_menu(61.0));
    assert!(Scene::notices(&controller).is_empty());
    assert!(controller.snapshot().library_read_only);

    controller.apply(LibraryAction::CreateProject("Castle".to_string()));
    controller.shutdown();
    assert!(data.path().join("library.hcl").is_dir());
}

#[test]
fn an_unreadable_marker_file_is_read_only_while_its_world_is_loaded() {
    let data = tempfile::tempdir().unwrap();
    let key = WorldKey { world: "W1".to_string(), dimension: 0 };
    let bad = data.path().join("markers").join(format!("{}.hcl", key.file_stem()));
    std::fs::create_dir_all(&bad).unwrap();
    let mut controller = ControllerHarness::start(data.path());
    Scene::import(&mut controller, data.path(), "tower", &Scene::stone());
    assert!(!controller.snapshot().markers_read_only);

    controller.tick(&Frames::in_world("W1", 20.0));
    let snapshot = controller.snapshot();
    assert!(snapshot.markers_read_only);
    assert!(snapshot.notices.contains(&ImportNotice::MarkersUnavailable));

    // The world still works in memory, but nothing is written over the unreadable file.
    Scene::place(&mut controller, "Home");
    assert_eq!(controller.snapshot().markers.len(), 1);
    controller.shutdown();
    assert!(bad.is_dir());

    // Another world loads fine, and the flag follows the world that is loaded.
    let mut second = ControllerHarness::start(data.path());
    second.tick(&Frames::in_world("W2", 1.0));
    assert!(!second.snapshot().markers_read_only);
    second.tick(&Frames::in_world("W1", 2.0));
    assert!(second.snapshot().markers_read_only);
    second.tick(&Frames::in_world("W2", 3.0));
    assert!(!second.snapshot().markers_read_only);
    second.tick(&Frames::main_menu(4.0));
    assert!(!second.snapshot().markers_read_only);
}

#[test]
fn failed_saves_wait_longer_each_time_and_log_once() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    controller.tick(&Frames::main_menu(0.0));
    // A folder in the way of the temporary file makes every write fail.
    let blocker = data.path().join("library.hcl.tmp");
    std::fs::create_dir_all(&blocker).unwrap();

    controller.apply(LibraryAction::CreateProject("Castle".to_string()));
    controller.tick(&Frames::main_menu(1.0));
    ControllerHarness::wait_for_failures(&controller, 1);
    // The tick that sees the failure sets a wait of 2 seconds.
    controller.tick(&Frames::main_menu(1.1));
    controller.tick(&Frames::main_menu(2.0));
    controller.tick(&Frames::main_menu(3.0));
    assert_eq!(controller.store_failures(), 1);

    controller.tick(&Frames::main_menu(3.2));
    ControllerHarness::wait_for_failures(&controller, 2);
    controller.tick(&Frames::main_menu(3.3));
    // Two failures in a row wrote one log line, and the next wait is 4 seconds.
    assert_eq!(controller.store_failure_logs(), 1);
    controller.tick(&Frames::main_menu(7.0));
    assert_eq!(controller.store_failures(), 2);

    // Once the disk works again, the retry saves the newest state.
    std::fs::remove_dir_all(&blocker).unwrap();
    controller.tick(&Frames::main_menu(7.4));
    ControllerHarness::flush(&controller);
    let saved = LibraryStore::new(data.path().to_path_buf()).load().unwrap();
    assert_eq!(saved.projects.len(), 1);
    assert_eq!(controller.store_failures(), 2);
}

#[test]
fn a_full_reader_queue_gives_a_busy_notice() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = StratumController::with_materials_worker(
        data.path().to_path_buf(),
        StratumConfig::default(),
        MaterialsWorker::without_thread(),
    );
    Scene::import(&mut controller, data.path(), "tower", &Scene::stone());
    let id = controller.snapshot().unsorted[0].id;

    // Each request cancels the one before, but the reads already queued still count against the reader.
    for _ in 0..MaterialsWorker::QUEUE {
        controller.apply(LibraryAction::LoadMaterials(MaterialScope::Structure(id)));
    }
    assert!(!Scene::notices(&controller).contains(&ImportNotice::MaterialsBusy));
    assert_eq!(controller.snapshot().materials_loading, Some(MaterialScope::Structure(id)));

    controller.apply(LibraryAction::LoadMaterials(MaterialScope::Structure(id)));
    assert!(Scene::notices(&controller).contains(&ImportNotice::MaterialsBusy));
    assert_eq!(controller.snapshot().materials_failed, Some(MaterialScope::Structure(id)));
    assert_eq!(controller.snapshot().materials_loading, None);
}

#[test]
fn shutdown_returns_within_its_budget() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    controller.apply(LibraryAction::CreateProject("Castle".to_string()));
    let started = std::time::Instant::now();
    controller.shutdown_within(Duration::from_secs(30));
    assert!(started.elapsed() < Duration::from_secs(30));
    assert_eq!(LibraryStore::new(data.path().to_path_buf()).load().unwrap().projects.len(), 1);
}

struct GhostConfigs;

impl GhostConfigs {
    fn standard() -> StratumConfig {
        StratumConfig::default()
    }

    fn full() -> StratumConfig {
        let mut config = StratumConfig::default();
        config.ghost.miniature = false;
        config
    }

    // A mesh budget that holds four slots.
    fn tight() -> StratumConfig {
        let mut config = StratumConfig::default();
        config.ghost.mesh_budget_mb = 1;
        config
    }
}

#[test]
fn activating_a_marker_loads_its_structures_then_draws() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = GhostScene::start(data.path(), GhostConfigs::standard(), [2, 2, 2]);

    GhostScene::tick_until_jobs(&mut controller, 2.0);

    let frame = controller.ghost_frame();
    assert!(!frame.jobs.is_empty());
    assert!(frame.palette.is_some());
    assert_eq!(frame.opacity, 0.5);
    let full_slots: Vec<u32> =
        frame.jobs.iter().filter(|job| GhostScene::is_full(job.key.part)).map(|job| job.slot).collect();
    assert!(!full_slots.is_empty());

    GhostScene::settle(&mut controller, 2.0);
    let frame = controller.ghost_frame();
    assert!(!frame.draws.is_empty());
    let full: Vec<_> = frame.draws.iter().filter(|draw| full_slots.contains(&draw.slot)).collect();
    assert!(!full.is_empty());
    assert!(full.iter().all(|draw| draw.scale == 1.0));
}

#[test]
fn deactivating_clears_in_the_next_frame() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = GhostScene::start(data.path(), GhostConfigs::standard(), [2, 2, 2]);
    GhostScene::tick_until_jobs(&mut controller, 2.0);
    GhostScene::settle(&mut controller, 2.0);
    assert!(!controller.ghost_frame().draws.is_empty());

    controller.apply(LibraryAction::DeactivateMarker);
    controller.tick(&Frames::in_world(GhostScene::WORLD, 3.0));

    let frame = controller.ghost_frame();
    assert!(frame.clear_all);
    assert!(frame.draws.is_empty());
    assert!(frame.jobs.is_empty());
}

#[test]
fn world_exit_clears() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = GhostScene::start(data.path(), GhostConfigs::standard(), [2, 2, 2]);
    GhostScene::tick_until_jobs(&mut controller, 2.0);
    GhostScene::settle(&mut controller, 2.0);

    controller.tick(&Frames::main_menu(3.0));

    let frame = controller.ghost_frame();
    assert!(frame.clear_all);
    assert!(frame.draws.is_empty());
    assert!(frame.jobs.is_empty());
}

#[test]
fn hidden_marker_and_lost_lease_draw_nothing_but_stay_active() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = GhostScene::start(data.path(), GhostConfigs::standard(), [2, 2, 2]);
    GhostScene::tick_until_jobs(&mut controller, 2.0);
    GhostScene::settle(&mut controller, 2.0);
    assert!(!controller.ghost_frame().draws.is_empty());
    let generation = controller.ghost_frame().generation;

    let live = controller.ghost_telemetry().sections_live;

    controller.set_ghost_allowed(false);
    controller.tick(&Frames::in_world(GhostScene::WORLD, 3.0));

    // the ghost stops in this frame, and its meshes stay for the next show
    let frame = controller.ghost_frame();
    assert!(!frame.clear_all);
    assert!(frame.draws.is_empty());
    assert!(frame.jobs.is_empty());
    assert!(frame.frees.is_empty());
    assert_eq!(frame.generation, generation);
    assert_eq!(controller.ghost_telemetry().sections_live, live);
    assert!(controller.snapshot().markers[0].active);

    controller.set_ghost_allowed(true);
    controller.tick(&Frames::in_world(GhostScene::WORLD, 4.0));

    let frame = controller.ghost_frame();
    assert!(!frame.clear_all);
    assert!(frame.palette.is_none());
    assert!(frame.jobs.is_empty());
    assert!(!frame.draws.is_empty());
    assert_eq!(frame.generation, generation);
}

#[test]
fn a_revoked_lease_clears_the_slots_and_the_next_show_builds_them_again() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = GhostScene::start(data.path(), GhostConfigs::standard(), [2, 2, 2]);
    GhostScene::tick_until_jobs(&mut controller, 2.0);
    GhostScene::settle(&mut controller, 2.0);
    let generation = controller.ghost_frame().generation;

    controller.ghost_revoked();
    controller.set_ghost_allowed(false);
    controller.tick(&Frames::in_world(GhostScene::WORLD, 3.0));

    let frame = controller.ghost_frame();
    assert!(frame.clear_all);
    assert!(frame.draws.is_empty());
    assert_eq!(controller.ghost_telemetry().sections_live, 0);

    controller.tick(&Frames::in_world(GhostScene::WORLD, 3.0));
    assert!(!controller.ghost_frame().clear_all);

    controller.set_ghost_allowed(true);
    controller.tick(&Frames::in_world(GhostScene::WORLD, 4.0));

    let frame = controller.ghost_frame();
    assert!(frame.palette.is_some());
    assert!(!frame.jobs.is_empty());
    assert_ne!(frame.generation, generation);
}

#[test]
fn a_new_draw_distance_applies_live() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = GhostScene::start(data.path(), GhostConfigs::full(), [48, 1, 1]);
    GhostScene::tick_until_jobs(&mut controller, 2.0);
    GhostScene::settle(&mut controller, 2.0);
    // three sections and the marker
    assert_eq!(controller.ghost_frame().draws.len(), 4);

    let mut config = GhostConfigs::full().ghost;
    config.draw_distance = 1;
    controller.set_ghost_config(config);
    controller.tick(&Frames::in_world(GhostScene::WORLD, 2.0));

    let frame = controller.ghost_frame();
    assert_eq!(frame.draws.len(), 3);
    assert_eq!(frame.frees.len(), 1);
    assert!(!frame.clear_all);
}

#[test]
fn the_world_epoch_moves_only_when_the_world_changes() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = GhostScene::start(data.path(), GhostConfigs::standard(), [2, 2, 2]);
    GhostScene::tick_until_jobs(&mut controller, 2.0);
    let epoch = controller.ghost_frame().world_epoch;
    assert!(controller.ghost_frame().in_world);
    GhostScene::settle(&mut controller, 2.0);
    controller.apply(LibraryAction::DeactivateMarker);
    controller.tick(&Frames::in_world(GhostScene::WORLD, 2.0));
    assert_eq!(controller.ghost_frame().world_epoch, epoch);

    controller.tick(&Frames::main_menu(3.0));
    let left = controller.ghost_frame().world_epoch;
    assert_ne!(left, epoch);
    assert!(!controller.ghost_frame().in_world);
    controller.tick(&Frames::main_menu(3.0));
    assert_eq!(controller.ghost_frame().world_epoch, left);

    controller.tick(&Frames::in_world(GhostScene::WORLD, 4.0));
    assert_ne!(controller.ghost_frame().world_epoch, left);
    assert!(controller.ghost_frame().in_world);
}

#[test]
fn a_marker_saved_as_hidden_draws_nothing_but_is_active() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = GhostScene::start(data.path(), GhostConfigs::standard(), [2, 2, 2]);
    controller.shutdown();
    drop(controller);
    let store = MarkerStore::new(data.path().join("markers"));
    let mut set = store.load(&WorldKey { world: GhostScene::WORLD.to_string(), dimension: 0 }).unwrap();
    set.markers[0].view.shown = false;
    let (path, bytes) = store.encode(&set).unwrap();
    std::fs::write(path, bytes).unwrap();

    let mut controller = ControllerHarness::start(data.path());
    for step in 0..20 {
        controller.tick(&Frames::in_world(GhostScene::WORLD, 2.0 + f64::from(step)));
        std::thread::sleep(Duration::from_millis(5));
        let frame = controller.ghost_frame();
        assert!(frame.jobs.is_empty());
        assert!(frame.draws.is_empty());
    }

    assert!(controller.snapshot().markers[0].active);
    assert_eq!(controller.ghost_telemetry().sections_live, 0);
}

#[test]
fn failed_load_draws_nothing_and_does_not_panic() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = GhostScene::start(data.path(), GhostConfigs::standard(), [2, 2, 2]);
    for entry in std::fs::read_dir(data.path().join("structures")).unwrap() {
        std::fs::remove_file(entry.unwrap().path()).unwrap();
    }

    let start = std::time::Instant::now();
    while controller.ghost_telemetry().failed_loads == 0 {
        assert!(start.elapsed() < Duration::from_secs(60), "the load did not fail");
        controller.tick(&Frames::in_world(GhostScene::WORLD, 2.0));
        std::thread::sleep(Duration::from_millis(2));
    }
    for _ in 0..5 {
        controller.tick(&Frames::in_world(GhostScene::WORLD, 2.0));
        assert!(controller.ghost_frame().jobs.is_empty());
        assert!(controller.ghost_frame().draws.is_empty());
    }

    let id = GhostScene::structure_id(&controller);
    controller.apply(LibraryAction::LoadMaterials(MaterialScope::Structure(id)));
    ControllerHarness::tick_until_materials_ready(&mut controller, &Frames::in_world(GhostScene::WORLD, 3.0));
    assert_eq!(controller.snapshot().materials_failed, Some(MaterialScope::Structure(id)));
    assert!(controller.snapshot().materials.is_none());
}

#[test]
fn cut_raises_one_notice_per_generation() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = GhostScene::start(data.path(), GhostConfigs::tight(), [96, 1, 1]);
    let cuts = |controller: &StratumController| {
        Scene::notices(controller).iter().filter(|notice| **notice == ImportNotice::GhostCut).count()
    };

    GhostScene::tick_until_jobs(&mut controller, 2.0);
    for _ in 0..5 {
        controller.tick(&Frames::in_world(GhostScene::WORLD, 2.0));
    }
    assert_eq!(cuts(&controller), 1);
    assert!(controller.ghost_telemetry().cut);

    let id = controller.snapshot().markers[0].id;
    controller.apply(LibraryAction::DeactivateMarker);
    controller.tick(&Frames::in_world(GhostScene::WORLD, 3.0));
    controller.apply(LibraryAction::ActivateMarker(id));
    GhostScene::tick_until_jobs(&mut controller, 3.0);
    for _ in 0..5 {
        controller.tick(&Frames::in_world(GhostScene::WORLD, 3.0));
    }

    assert_eq!(cuts(&controller), 2);
}

#[test]
fn ghost_loads_do_not_disturb_material_lists() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = GhostScene::start(data.path(), GhostConfigs::standard(), [2, 2, 2]);
    controller.tick(&Frames::in_world(GhostScene::WORLD, 2.0));
    let id = GhostScene::structure_id(&controller);

    controller.apply(LibraryAction::LoadMaterials(MaterialScope::Structure(id)));
    ControllerHarness::tick_until_materials_ready(&mut controller, &Frames::in_world(GhostScene::WORLD, 2.0));
    GhostScene::tick_until_jobs(&mut controller, 2.0);

    let snapshot = controller.snapshot();
    assert!(snapshot.materials.is_some());
    assert!(snapshot.materials_failed.is_none());
}

#[test]
fn block_change_rebuilds_its_section_and_edge_neighbour() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = GhostScene::start(data.path(), GhostConfigs::standard(), [32, 1, 1]);
    GhostScene::tick_until_jobs(&mut controller, 2.0);
    assert_eq!(GhostScene::job_xs(&controller), [0, 1]);
    GhostScene::rest(&mut controller, 2.0);

    controller.ghost_block_changed([15, 64, 0]);
    controller.tick(&Frames::in_world(GhostScene::WORLD, 2.0));
    assert_eq!(GhostScene::job_xs(&controller), [0, 1]);
    GhostScene::settle(&mut controller, 2.0);

    controller.ghost_block_changed([5, 64, 0]);
    controller.tick(&Frames::in_world(GhostScene::WORLD, 2.0));
    assert_eq!(GhostScene::job_xs(&controller), [0]);
    GhostScene::settle(&mut controller, 2.0);

    controller.ghost_block_changed([100, 64, 0]);
    controller.ghost_block_changed([-3, 64, 0]);
    controller.ghost_block_changed([5, 90, 0]);
    controller.tick(&Frames::in_world(GhostScene::WORLD, 2.0));
    assert!(controller.ghost_frame().jobs.is_empty());
}

#[test]
fn block_change_flood_invalidates_the_build_once() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = GhostScene::start(data.path(), GhostConfigs::standard(), [48, 1, 1]);
    GhostScene::tick_until_jobs(&mut controller, 2.0);
    GhostScene::rest(&mut controller, 2.0);

    for _ in 0..5000 {
        controller.ghost_block_changed([1, 64, 0]);
    }
    controller.tick(&Frames::in_world(GhostScene::WORLD, 2.0));

    assert_eq!(GhostScene::job_xs(&controller), [0]);
    assert_eq!(controller.ghost_telemetry().block_change_overflows, 1);
    GhostScene::rest(&mut controller, 2.0);
}

#[test]
fn changes_past_the_capacity_mark_only_the_sections_their_box_touches() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = GhostScene::start(data.path(), GhostConfigs::standard(), [80, 1, 1]);
    GhostScene::tick_until_jobs(&mut controller, 2.0);
    GhostScene::rest(&mut controller, 2.0);

    for _ in 0..GhostService::BLOCK_CHANGE_CAPACITY {
        controller.ghost_block_changed([1, 64, 0]);
    }
    // past the capacity: the box from x 40 to x 41 lies in section 2 and in no padding of sections 1 or 3
    controller.ghost_block_changed([40, 64, 0]);
    controller.ghost_block_changed([41, 64, 0]);
    controller.tick(&Frames::in_world(GhostScene::WORLD, 2.0));

    assert_eq!(GhostScene::job_xs(&controller), [0, 2]);
    assert_eq!(controller.ghost_telemetry().block_change_overflows, 1);
}

#[test]
fn block_changes_up_to_the_capacity_do_not_overflow() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = GhostScene::start(data.path(), GhostConfigs::standard(), [48, 1, 1]);
    GhostScene::tick_until_jobs(&mut controller, 2.0);
    GhostScene::settle(&mut controller, 2.0);

    for _ in 0..4096 {
        controller.ghost_block_changed([1, 64, 0]);
    }
    controller.tick(&Frames::in_world(GhostScene::WORLD, 2.0));

    assert_eq!(GhostScene::job_xs(&controller), [0]);
    assert_eq!(controller.ghost_telemetry().block_change_overflows, 0);
}

#[test]
fn rejoining_a_world_resends_the_palette() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = GhostScene::start(data.path(), GhostConfigs::standard(), [2, 2, 2]);
    GhostScene::tick_until_jobs(&mut controller, 2.0);
    let generation = controller.ghost_frame().generation;
    GhostScene::settle(&mut controller, 2.0);

    controller.tick(&Frames::main_menu(3.0));
    GhostScene::tick_until_palette(&mut controller, 4.0);

    assert_ne!(controller.ghost_frame().generation, generation);
    assert!(controller.ghost_frame().palette.is_some());
}

#[test]
fn a_world_reset_report_starts_a_new_generation_with_the_palette_and_a_clear() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = GhostScene::start(data.path(), GhostConfigs::standard(), [2, 2, 2]);
    GhostScene::tick_until_jobs(&mut controller, 2.0);
    let generation = controller.ghost_frame().generation;
    GhostScene::settle(&mut controller, 2.0);
    assert!(controller.ghost_frame().palette.is_none());

    let mut report = GhostReport::new();
    report.world_reset = true;
    controller.apply_ghost_report(&report);
    controller.tick(&Frames::in_world(GhostScene::WORLD, 2.0));

    let frame = controller.ghost_frame();
    assert_ne!(frame.generation, generation);
    assert!(frame.clear_all);
    assert!(frame.palette.is_some());
    assert!(!frame.jobs.is_empty());
}

// Steps for the picker: a request for `destination`, answered with `files`.
struct Pick;

impl Pick {
    fn answer(controller: &mut StratumController, destination: Option<ProjectId>, files: Vec<std::path::PathBuf>) {
        controller.apply(LibraryAction::RequestFilePicker(destination));
        assert!(controller.take_picker_request());
        controller.files_picked(files);
        ControllerHarness::settle(controller, &Frames::main_menu(0.0));
    }

    fn files(dir: &std::path::Path, names: &[&str]) -> Vec<std::path::PathBuf> {
        names
            .iter()
            .map(|name| ControllerHarness::picked_file(dir, &format!("{name}.mcstructure"), &Scene::stone()))
            .collect()
    }

    fn project(controller: &mut StratumController, name: &str) -> ProjectId {
        controller.apply(LibraryAction::CreateProject(name.to_string()));
        controller.snapshot().projects.iter().find(|row| row.name == name).unwrap().id
    }

    fn ids_in(controller: &StratumController, project: ProjectId) -> Vec<StructureId> {
        let snapshot = controller.snapshot();
        let row = snapshot.projects.iter().find(|row| row.id == project).unwrap();
        let mut ids: Vec<StructureId> = row.structures.iter().map(|structure| structure.id).collect();
        ids.sort();
        ids
    }

    fn unsorted_ids(controller: &StratumController) -> Vec<StructureId> {
        let mut ids: Vec<StructureId> = controller.snapshot().unsorted.iter().map(|structure| structure.id).collect();
        ids.sort();
        ids
    }

    fn sorted(mut ids: Vec<StructureId>) -> Vec<StructureId> {
        ids.sort();
        ids
    }
}

#[test]
fn pick_from_a_project_lands_in_that_project() {
    let data = tempfile::tempdir().unwrap();
    let files = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    let project = Pick::project(&mut controller, "Castle");
    Pick::answer(&mut controller, Some(project), Pick::files(files.path(), &["a", "b"]));

    let snapshot = controller.snapshot();
    let pick = snapshot.latest_pick.clone().unwrap();
    assert_eq!(Pick::ids_in(&controller, project).len(), 2);
    assert!(snapshot.unsorted.is_empty());
    assert_eq!(pick.destination, Some(project));
    assert_eq!(pick.structures.len(), 2);
    assert_eq!(Pick::sorted(pick.structures), Pick::ids_in(&controller, project));
}

#[test]
fn pick_without_a_project_lands_in_unsorted() {
    let data = tempfile::tempdir().unwrap();
    let files = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    Pick::answer(&mut controller, None, Pick::files(files.path(), &["a", "b"]));

    let pick = controller.snapshot().latest_pick.clone().unwrap();
    assert_eq!(pick.destination, None);
    assert_eq!(Pick::sorted(pick.structures), Pick::unsorted_ids(&controller));
    assert_eq!(controller.snapshot().unsorted.len(), 2);
}

#[test]
fn project_deleted_during_the_picker_falls_back_to_unsorted() {
    let data = tempfile::tempdir().unwrap();
    let files = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    let project = Pick::project(&mut controller, "Castle");
    controller.apply(LibraryAction::RequestFilePicker(Some(project)));
    assert!(controller.take_picker_request());
    controller.apply(LibraryAction::DeleteProject(project));
    controller.files_picked(Pick::files(files.path(), &["a"]));
    ControllerHarness::settle(&mut controller, &Frames::main_menu(0.0));

    let snapshot = controller.snapshot();
    let pick = snapshot.latest_pick.clone().unwrap();
    assert_eq!(pick.destination, None);
    assert_eq!(snapshot.unsorted.len(), 1);
    assert_eq!(pick.structures, vec![snapshot.unsorted[0].id]);
}

#[test]
fn next_pick_replaces_the_latest_pick() {
    let data = tempfile::tempdir().unwrap();
    let files = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    Pick::answer(&mut controller, None, Pick::files(files.path(), &["a"]));
    let first = controller.snapshot().latest_pick.clone().unwrap();
    Pick::answer(&mut controller, None, Pick::files(files.path(), &["b", "c"]));
    let second = controller.snapshot().latest_pick.clone().unwrap();

    assert!(second.serial > first.serial);
    assert_eq!(second.structures.len(), 2);
    assert!(second.structures.iter().all(|id| !first.structures.contains(id)));
}

#[test]
fn failed_pick_publishes_no_structures() {
    let data = tempfile::tempdir().unwrap();
    let files = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    let junk = ControllerHarness::picked_file(files.path(), "junk.mcstructure", b"not a structure");
    Pick::answer(&mut controller, None, vec![junk]);

    let snapshot = controller.snapshot();
    assert!(snapshot.latest_pick.clone().unwrap().structures.is_empty());
    assert!(snapshot.unsorted.is_empty());
}

#[test]
fn inbox_import_does_not_touch_the_latest_pick() {
    let data = tempfile::tempdir().unwrap();
    let files = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    Pick::answer(&mut controller, None, Pick::files(files.path(), &["a"]));
    let before = controller.snapshot().latest_pick.clone();

    ControllerHarness::inbox_file(data.path(), "castle.mcstructure", &Scene::stone());
    controller.tick(&Frames::main_menu(10.0));
    controller.tick(&Frames::main_menu(12.0));
    ControllerHarness::settle(&mut controller, &Frames::main_menu(12.0));

    assert_eq!(controller.snapshot().unsorted.len(), 2);
    assert_eq!(controller.snapshot().latest_pick, before);
}

#[test]
fn pick_places_no_marker() {
    let data = tempfile::tempdir().unwrap();
    let files = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    controller.tick(&Frames::in_world("W1", 1.0));
    let before = controller.snapshot().markers.clone();
    controller.apply(LibraryAction::RequestFilePicker(None));
    assert!(controller.take_picker_request());
    controller.files_picked(Pick::files(files.path(), &["a"]));
    ControllerHarness::settle(&mut controller, &Frames::in_world("W1", 2.0));

    assert_eq!(controller.snapshot().unsorted.len(), 1);
    assert_eq!(controller.snapshot().markers, before);
}

#[test]
fn picker_busy_spans_request_to_answer() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    assert!(!controller.picker_busy());
    controller.apply(LibraryAction::RequestFilePicker(None));
    assert!(controller.picker_busy());
    assert!(controller.take_picker_request());
    assert!(controller.picker_busy());
    controller.files_picked(Vec::new());
    assert!(!controller.picker_busy());
}

#[test]
fn queue_full_pick_records_only_accepted_sources() {
    let data = tempfile::tempdir().unwrap();
    let files = tempfile::tempdir().unwrap();
    let mut controller = StratumController::new(data.path().to_path_buf(), ControllerHarness::config(64, 1));
    let names: Vec<String> = (0..40).map(|index| format!("s{index}")).collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    controller.apply(LibraryAction::RequestFilePicker(None));
    assert!(controller.take_picker_request());
    controller.files_picked(Pick::files(files.path(), &names));
    let accepted = controller.pending_imports();
    assert!(accepted > 0 && accepted < names.len());
    ControllerHarness::settle(&mut controller, &Frames::main_menu(0.0));

    let snapshot = controller.snapshot();
    assert!(snapshot.notices.contains(&ImportNotice::ImportQueueFull));
    let first = snapshot.latest_pick.clone().unwrap();
    assert_eq!(first.structures.len(), accepted);
    assert_eq!(snapshot.unsorted.len(), accepted);
    assert_eq!(controller.pending_imports(), 0);

    // Nothing stays recorded as in flight: a later pick holds only its own structure.
    Pick::answer(&mut controller, None, Pick::files(files.path(), &["extra"]));
    let second = controller.snapshot().latest_pick.clone().unwrap();
    assert_eq!(second.structures.len(), 1);
    assert!(!first.structures.contains(&second.structures[0]));
}

#[test]
fn hud_action_on_the_active_marker_is_saved() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    Scene::import(&mut controller, data.path(), "tower", &Scene::stone());
    controller.tick(&Frames::at("W1", [10, 70, -20], 1.0));
    Scene::place(&mut controller, "Home");

    controller.apply_hud(HudAction::Move(Axis::X, 1));
    controller.apply_hud(HudAction::Rotate(1));
    controller.apply_hud(HudAction::OpacityUp);
    controller.apply_hud(HudAction::LayerUp);
    controller.apply_hud(HudAction::LayerUp);
    controller.shutdown();

    let saved = MarkerStore::new(data.path().join("markers"))
        .load(&WorldKey { world: "W1".to_string(), dimension: 0 })
        .unwrap();
    let marker = &saved.markers[0];
    assert_eq!(marker.origin, [11, 70, -20]);
    assert_eq!(marker.rotation, Rotation::R90);
    assert_eq!(marker.view.opacity_step, 3);
    // the stone structure is 2 layers tall: a new marker shows the whole build, so two presses reach layer 1
    assert_eq!(marker.view.shown_layer(2), Some(1));
}

#[test]
fn hud_action_without_an_active_marker_does_nothing() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    Scene::import(&mut controller, data.path(), "tower", &Scene::stone());
    controller.tick(&Frames::at("W1", [10, 70, -20], 1.0));
    Scene::place(&mut controller, "Home");
    controller.apply(LibraryAction::DeactivateMarker);

    controller.apply_hud(HudAction::Move(Axis::X, 1));
    controller.apply_hud(HudAction::ToggleShown);
    controller.shutdown();

    let saved = MarkerStore::new(data.path().join("markers"))
        .load(&WorldKey { world: "W1".to_string(), dimension: 0 })
        .unwrap();
    assert_eq!(saved.markers[0].origin, [10, 70, -20]);
    assert!(saved.markers[0].view.shown);
}

use stratum_core::structs::marker::ViewMode;

// Project scenes for the HUD tests that go through `StratumController::apply_hud`.
struct HudScene;

impl HudScene {
    // A normal project of three stacked-height structures a (2), b (3) and c (5), with a marker on
    // the project placed and active.
    fn normal_project(data: &std::path::Path) -> StratumController {
        let mut controller = ControllerHarness::start(data);
        for (name, height) in [("a", 2), ("b", 3), ("c", 5)] {
            Scene::import(&mut controller, data, name, &StructureFixture::full([1, height, 1]).bytes());
        }
        controller.apply(LibraryAction::CreateProject("Keep".to_string()));
        let project = controller.snapshot().projects[0].id;
        for row in controller.snapshot().unsorted.clone() {
            controller.apply(LibraryAction::MoveStructure(row.id, Some(project)));
        }
        controller.tick(&Frames::in_world("W1", 1.0));
        controller
            .apply(LibraryAction::PlaceMarker { name: "Home".to_string(), target: MarkerTarget::Project(project) });
        controller
    }

    // Two pieces on top of each other, 2 and 3 blocks tall, so the build is 5 layers.
    fn big_build(data: &std::path::Path) -> StratumController {
        let mut controller = ControllerHarness::start(data);
        controller.apply(LibraryAction::CreateProject("Tower".to_string()));
        let project = controller.snapshot().projects[0].id;
        controller.apply(LibraryAction::SetProjectKind(project, ProjectKind::BigBuild));
        let low = StructureFixture::full([2, 2, 2]).origin([0, 60, 0]);
        let high = StructureFixture::full([2, 3, 2]).origin([0, 62, 0]);
        Scene::import(&mut controller, data, "low", &low.bytes());
        Scene::import(&mut controller, data, "high", &high.bytes());
        for row in controller.snapshot().unsorted.clone() {
            controller.apply(LibraryAction::MoveStructure(row.id, Some(project)));
        }
        controller.tick(&Frames::in_world("W1", 1.0));
        controller
            .apply(LibraryAction::PlaceMarker { name: "Tower".to_string(), target: MarkerTarget::Project(project) });
        controller
    }

    fn press(controller: &mut StratumController, action: HudAction, times: usize) {
        for _ in 0..times {
            controller.apply_hud(action);
        }
    }

    // The marker as saved on shutdown.
    fn saved(mut controller: StratumController, data: &std::path::Path) -> stratum_core::structs::marker::Marker {
        controller.shutdown();
        let key = WorldKey { world: "W1".to_string(), dimension: 0 };
        MarkerStore::new(data.join("markers")).load(&key).unwrap().markers.remove(0)
    }
}

#[test]
fn structure_next_wraps_from_the_last_structure_of_a_normal_project() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = HudScene::normal_project(data.path());

    HudScene::press(&mut controller, HudAction::StructureNext, 2);
    let marker = HudScene::saved(controller, data.path());
    assert_eq!(marker.view.selected, 2);

    let data = tempfile::tempdir().unwrap();
    let mut controller = HudScene::normal_project(data.path());
    HudScene::press(&mut controller, HudAction::StructureNext, 3);
    let marker = HudScene::saved(controller, data.path());
    assert_eq!(marker.view.selected, 0);
}

#[test]
fn structure_prev_wraps_from_the_first_structure_of_a_normal_project() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = HudScene::normal_project(data.path());

    HudScene::press(&mut controller, HudAction::StructurePrev, 1);

    let marker = HudScene::saved(controller, data.path());
    assert_eq!(marker.view.selected, 2);
}

#[test]
fn layer_up_reaches_whole_after_the_top_of_the_selected_structure() {
    // a new marker shows Whole. a is 2 layers tall: the second press reaches layer 1, the third goes to Whole.
    let data = tempfile::tempdir().unwrap();
    let mut controller = HudScene::normal_project(data.path());
    HudScene::press(&mut controller, HudAction::LayerUp, 2);
    let marker = HudScene::saved(controller, data.path());
    assert_eq!(marker.view.shown_layer(2), Some(1));

    let data = tempfile::tempdir().unwrap();
    let mut controller = HudScene::normal_project(data.path());
    HudScene::press(&mut controller, HudAction::LayerUp, 3);
    let marker = HudScene::saved(controller, data.path());
    assert_eq!(marker.view.view_mode, ViewMode::Whole);

    // c is 5 layers tall: five presses show layer 4, a sixth shows Whole.
    let data = tempfile::tempdir().unwrap();
    let mut controller = HudScene::normal_project(data.path());
    HudScene::press(&mut controller, HudAction::StructureNext, 2);
    HudScene::press(&mut controller, HudAction::LayerUp, 5);
    let marker = HudScene::saved(controller, data.path());
    assert_eq!(marker.view.selected, 2);
    assert_eq!(marker.view.shown_layer(5), Some(4));

    let data = tempfile::tempdir().unwrap();
    let mut controller = HudScene::normal_project(data.path());
    HudScene::press(&mut controller, HudAction::StructureNext, 2);
    HudScene::press(&mut controller, HudAction::LayerUp, 6);
    let marker = HudScene::saved(controller, data.path());
    assert_eq!(marker.view.view_mode, ViewMode::Whole);
}

#[test]
fn layer_up_uses_the_height_of_a_big_build_after_a_piece_is_added() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = HudScene::big_build(data.path());
    let project = controller.snapshot().projects[0].id;
    let top = StructureFixture::full([2, 3, 2]).origin([0, 65, 0]);
    Scene::import(&mut controller, data.path(), "top", &top.bytes());
    for row in controller.snapshot().unsorted.clone() {
        controller.apply(LibraryAction::MoveStructure(row.id, Some(project)));
    }
    controller.tick(&Frames::in_world("W1", 1.5));

    HudScene::press(&mut controller, HudAction::LayerUp, 6);
    let marker = HudScene::saved(controller, data.path());
    assert_eq!(marker.view.shown_layer(8), Some(5), "the build is now 8 layers tall");
}

#[test]
fn layer_up_reaches_whole_after_the_full_height_of_a_big_build() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = HudScene::big_build(data.path());
    HudScene::press(&mut controller, HudAction::LayerUp, 5);
    let marker = HudScene::saved(controller, data.path());
    assert_eq!(marker.view.shown_layer(5), Some(4));

    let data = tempfile::tempdir().unwrap();
    let mut controller = HudScene::big_build(data.path());
    HudScene::press(&mut controller, HudAction::LayerUp, 6);
    let marker = HudScene::saved(controller, data.path());
    assert_eq!(marker.view.view_mode, ViewMode::Whole);
}

#[test]
fn structure_keys_do_not_move_the_selection_of_a_big_build() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = HudScene::big_build(data.path());

    HudScene::press(&mut controller, HudAction::StructureNext, 2);

    let marker = HudScene::saved(controller, data.path());
    assert_eq!(marker.view.selected, 0);
}

#[test]
fn turned_copies_are_charged_to_the_structure_cache_budget() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = GhostScene::start(data.path(), GhostConfigs::standard(), [2, 2, 2]);
    GhostScene::tick_until_jobs(&mut controller, 2.0);
    let before = controller.ghost_telemetry();
    assert_eq!(before.turned_bytes, 0);
    assert_eq!(before.cache_resident_bytes, controller.cache_resident_bytes());

    controller.apply_hud(HudAction::Rotate(1));
    let started = std::time::Instant::now();
    while controller.ghost_telemetry().turned_bytes == 0 {
        assert!(started.elapsed() < Duration::from_secs(60), "the turn did not arrive");
        controller.tick(&Frames::in_world(GhostScene::WORLD, 3.0));
        std::thread::sleep(Duration::from_millis(1));
    }
    let turned = controller.ghost_telemetry();
    assert_eq!(turned.turned_bytes, 16);
    assert_eq!(turned.cache_resident_bytes, before.cache_resident_bytes + 16);
    assert_eq!(turned.cache_budget_bytes, before.cache_budget_bytes);

    controller.apply_hud(HudAction::Rotate(-1));
    controller.tick(&Frames::in_world(GhostScene::WORLD, 4.0));
    let back = controller.ghost_telemetry();
    assert_eq!(back.turned_bytes, 0);
    assert_eq!(back.cache_resident_bytes, before.cache_resident_bytes);
}

// Controllers in a world with a marker on a structure or on a project.
struct RowScene;

impl RowScene {
    fn import(controller: &mut StratumController, dir: &std::path::Path, name: &str) {
        Self::import_sized(controller, dir, name, StructureFixture::full([1, 2, 1]));
    }

    fn import_sized(controller: &mut StratumController, dir: &std::path::Path, name: &str, fixture: StructureFixture) {
        let bytes = fixture.bytes();
        let path = ControllerHarness::picked_file(dir, &format!("{name}.mcstructure"), &bytes);
        controller.files_picked(vec![path]);
        ControllerHarness::settle(controller, &Frames::main_menu(0.0));
    }

    // A project of `count` structures with a marker on it.
    fn marked(data: &std::path::Path, count: usize, kind: ProjectKind) -> StratumController {
        let mut controller = ControllerHarness::start(data);
        for index in 0..count {
            Self::import(&mut controller, data, &format!("s{index}"));
        }
        controller.apply(LibraryAction::CreateProject("Keep".to_string()));
        let id = controller.snapshot().projects[0].id;
        controller.apply(LibraryAction::SetProjectKind(id, kind));
        for row in controller.snapshot().unsorted.clone() {
            controller.apply(LibraryAction::MoveStructure(row.id, Some(id)));
        }
        controller.tick(&Frames::in_world("W1", 1.0));
        let target = MarkerTarget::Project(id);
        controller.apply(LibraryAction::PlaceMarker { name: "Home".to_string(), target });
        controller
    }

    // One structure of 2 by 5 by 2 under a marker at the player's feet (0, 64, 0).
    fn structure_marker(data: &std::path::Path) -> StratumController {
        let mut controller = ControllerHarness::start(data);
        Self::import_sized(&mut controller, data, "tower", StructureFixture::full([2, 5, 2]));
        controller.tick(&Frames::in_world("W1", 1.0));
        let target = MarkerTarget::Structure(controller.snapshot().unsorted[0].id);
        controller.apply(LibraryAction::PlaceMarker { name: "Home".to_string(), target });
        controller.tick(&Frames::in_world("W1", 1.1));
        controller
    }

    // A big build of piece "west" at x 0 to 3 and piece "east" at x 4 to 7, under a marker at (0, 64, 0).
    fn big_build_marker(data: &std::path::Path) -> StratumController {
        let mut controller = ControllerHarness::start(data);
        Self::import_sized(&mut controller, data, "west", StructureFixture::full([4, 3, 4]).origin([0, 0, 0]));
        Self::import_sized(&mut controller, data, "east", StructureFixture::full([4, 3, 4]).origin([4, 0, 0]));
        controller.apply(LibraryAction::CreateProject("Keep".to_string()));
        let id = controller.snapshot().projects[0].id;
        controller.apply(LibraryAction::SetProjectKind(id, ProjectKind::BigBuild));
        for row in controller.snapshot().unsorted.clone() {
            controller.apply(LibraryAction::MoveStructure(row.id, Some(id)));
        }
        controller.tick(&Frames::in_world("W1", 1.0));
        controller.apply(LibraryAction::PlaceMarker { name: "Keep".to_string(), target: MarkerTarget::Project(id) });
        controller.tick(&Frames::in_world("W1", 1.1));
        controller
    }
}

#[test]
fn a_normal_project_of_two_structures_has_structures_to_step_through() {
    let data = tempfile::tempdir().unwrap();
    let controller = RowScene::marked(data.path(), 2, ProjectKind::Normal);

    assert!(controller.hud_has_structures());
}

#[test]
fn a_normal_project_of_one_structure_has_none() {
    let data = tempfile::tempdir().unwrap();
    let controller = RowScene::marked(data.path(), 1, ProjectKind::Normal);

    assert!(!controller.hud_has_structures());
}

#[test]
fn a_big_build_has_none() {
    let data = tempfile::tempdir().unwrap();
    let controller = RowScene::marked(data.path(), 2, ProjectKind::BigBuild);

    assert!(!controller.hud_has_structures());
}

#[test]
fn without_an_active_marker_there_are_none() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = RowScene::marked(data.path(), 2, ProjectKind::Normal);
    controller.apply(LibraryAction::DeactivateMarker);

    assert!(!controller.hud_has_structures());
}

#[test]
fn hud_row_follows_the_active_marker() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = RowScene::structure_marker(data.path());

    let hud = controller.snapshot().hud.clone().expect("an active marker has a card");
    assert_eq!(hud.marker_name, "Home");
    assert_eq!(hud.target_name, "tower");
    assert_eq!((hud.layers, hud.layer, hud.y_level, hud.y_range), (5, None, None, Some((64, 68))));
    assert_eq!((hud.position, hud.piece_name.as_deref()), (None, None));

    for _ in 0..5 {
        controller.apply_hud(HudAction::LayerUp);
    }
    controller.apply_hud(HudAction::OpacityUp);
    controller.apply_hud(HudAction::Move(Axis::X, -1));
    controller.apply_hud(HudAction::Rotate(1));
    controller.tick(&Frames::in_world("W1", 1.2));
    let hud = controller.snapshot().hud.clone().unwrap();
    assert_eq!((hud.layer, hud.y_level), (Some(4), Some(68)), "the top layer");
    assert_eq!((hud.opacity_step, hud.origin, hud.rotation), (3, [-1, 64, 0], Rotation::R90));

    controller.apply_hud(HudAction::LayerUp);
    controller.apply_hud(HudAction::ToggleShown);
    controller.tick(&Frames::in_world("W1", 1.3));
    let hud = controller.snapshot().hud.clone().unwrap();
    assert_eq!((hud.layer, hud.y_level, hud.y_range), (None, None, Some((64, 68))), "after the top, the whole build");
    assert!(!hud.shown);

    controller.apply(LibraryAction::DeactivateMarker);
    assert!(controller.snapshot().hud.is_none(), "no card without an active marker");
}

#[test]
fn hud_row_names_the_big_build_piece_at_or_nearest_the_feet() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = RowScene::big_build_marker(data.path());

    controller.tick(&Frames::at("W1", [5, 64, 1], 2.0));
    let hud = controller.snapshot().hud.clone().unwrap();
    assert_eq!(hud.piece_name.as_deref(), Some("east"), "the feet stand in the east piece");
    assert_eq!((hud.target_name.as_str(), hud.layers, hud.position), ("Keep", 3, None));

    controller.tick(&Frames::at("W1", [-30, 64, 2], 2.1));
    let hud = controller.snapshot().hud.clone().unwrap();
    assert_eq!(hud.piece_name.as_deref(), Some("west"), "the west piece is nearest");
}

#[test]
fn hud_row_counts_the_selected_structure_of_a_normal_project() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = RowScene::marked(data.path(), 3, ProjectKind::Normal);
    controller.apply_hud(HudAction::StructureNext);
    controller.tick(&Frames::in_world("W1", 2.0));

    let hud = controller.snapshot().hud.clone().unwrap();
    assert_eq!(hud.position, Some((2, 3)));
    assert_eq!(hud.target_name, "s1");
    assert!(hud.has_structures);
}

#[test]
fn hud_row_names_the_piece_at_the_turned_position_of_a_rotated_big_build() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = RowScene::big_build_marker(data.path());
    controller.apply_hud(HudAction::Rotate(1));

    controller.tick(&Frames::at("W1", [-2, 64, 5], 2.0));
    let hud = controller.snapshot().hud.clone().unwrap();
    assert_eq!(hud.rotation, Rotation::R90);
    assert_eq!(hud.piece_name.as_deref(), Some("east"), "the turned east piece covers x -3 to 0, z 4 to 7");

    controller.tick(&Frames::at("W1", [-2, 64, 1], 2.1));
    let hud = controller.snapshot().hud.clone().unwrap();
    assert_eq!(hud.piece_name.as_deref(), Some("west"), "the turned west piece covers x -3 to 0, z 0 to 3");
}

#[test]
fn a_newly_placed_marker_shows_the_whole_build() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = RowScene::structure_marker(data.path());

    let hud = controller.snapshot().hud.clone().unwrap();
    assert_eq!((hud.layer, hud.y_level), (None, None));
    assert_eq!(hud.y_range, Some((64, 68)));
    controller.shutdown();
    let key = WorldKey { world: "W1".to_string(), dimension: 0 };
    let saved = MarkerStore::new(data.path().join("markers")).load(&key).unwrap().markers.remove(0);
    assert_eq!((saved.view.view_mode, saved.view.layer), (ViewMode::Whole, 0));
}

#[test]
fn a_marker_whose_target_is_gone_has_no_layer_readout() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = RowScene::structure_marker(data.path());
    let id = controller.snapshot().unsorted[0].id;
    controller.apply(LibraryAction::DeleteStructure(id));
    controller.tick(&Frames::in_world("W1", 2.0));

    let hud = controller.snapshot().hud.clone().unwrap();
    assert_eq!((hud.layers, hud.layer, hud.y_range), (0, None, None));
}

#[test]
fn quick_mode_up_steps_the_layer_of_the_active_marker() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = RowScene::structure_marker(data.path());

    controller.hud_key(HudKey::Up);
    controller.tick(&Frames::in_world("W1", 1.2));

    let hud = controller.snapshot().hud.clone().unwrap();
    assert_eq!((hud.layer, hud.y_level), (Some(0), Some(64)));
}

#[test]
fn full_mode_walks_the_carousel_to_move_x_and_acts_on_it() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = RowScene::structure_marker(data.path());

    controller.hud_key(HudKey::Mode);
    for _ in 0..3 {
        controller.hud_key(HudKey::Right);
    }
    assert_eq!(controller.hud_state().mode, HudMode::Full);
    assert_eq!(controller.hud_state().control, HudControl::MoveX, "one structure, so Structure is skipped");

    controller.hud_key(HudKey::Up);
    controller.tick(&Frames::in_world("W1", 1.2));

    let hud = controller.snapshot().hud.clone().unwrap();
    assert_eq!(hud.origin, [1, 64, 0]);
}

#[test]
fn hud_keys_without_an_active_marker_move_the_mode_and_change_no_marker() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = RowScene::structure_marker(data.path());
    controller.apply(LibraryAction::DeactivateMarker);

    controller.hud_key(HudKey::Mode);
    controller.hud_key(HudKey::Up);
    controller.tick(&Frames::in_world("W1", 1.2));

    assert_eq!(controller.hud_state().mode, HudMode::Full);
    assert!(controller.snapshot().hud.is_none());
    let marker = HudScene::saved(controller, data.path());
    assert_eq!((marker.view.view_mode, marker.view.shown_layer(5)), (ViewMode::Whole, None));
    assert_eq!(marker.origin, [0, 64, 0]);
}

#[test]
fn ghost_telemetry_reports_miniature_slots() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = GhostScene::start(data.path(), GhostConfigs::standard(), [48, 1, 1]);
    GhostScene::tick_until_jobs(&mut controller, 2.0);
    let awaiting = controller.ghost_telemetry();
    assert_eq!((awaiting.miniature_slots, awaiting.miniature_sections, awaiting.miniature_bytes), (0, 0, 0));

    for _ in 0..16 {
        GhostScene::settle(&mut controller, 2.0);
    }

    let telemetry = controller.ghost_telemetry();
    assert_eq!(telemetry.miniature_slots, 1);
    assert_eq!(telemetry.miniature_sections, 3);
    assert_eq!(telemetry.miniature_bytes, 1000);
    assert!(telemetry.used_bytes > telemetry.miniature_bytes);
    assert!(!telemetry.miniature_over_limit);
}

use stratum_core::services::EasyPlaceService;
use stratum_core::structs::easy_place::{
    EasyPlaceNotice, GameMode, InventorySlot, SpotWorld, SwapAnswer, UseCommand, UseProbe,
};
use stratum_core::structs::material::ItemId;

// Steps for the easy place tests that go through `StratumController`: a flat build of stone cells
// along x from (0, 64, 0), seen from above.
struct UseScene;

impl UseScene {
    const DEADLINE: Duration = Duration::from_secs(60);
    const STONE: &'static [u8] = b"minecraft:stone";

    // A controller with a 3 by 1 by 1 stone ghost whose build is made.
    fn start(data: &std::path::Path) -> StratumController {
        let mut controller = GhostScene::start(data, StratumConfig::default(), [3, 1, 1]);
        GhostScene::tick_until_jobs(&mut controller, 2.0);
        controller
    }

    fn enable(controller: &mut StratumController) {
        controller.apply_hud(HudAction::ToggleEasyPlace);
        controller.tick(&Frames::in_world(GhostScene::WORLD, 2.0));
    }

    // Looks straight down at the cell column (x, z) from above the build.
    fn above(x: i32, z: i32) -> UseProbe {
        UseProbe {
            time_seconds: 2.0,
            pressed: false,
            held: true,
            eye: [f64::from(x) + 0.5, 70.5, f64::from(z) + 0.5],
            dir: [0.0, -1.0, 0.0],
            reach: 20.0,
            game_mode: GameMode::Survival,
            hit: None,
            body: None,
            swap_answer: SwapAnswer::None,
        }
    }

    // Looks away from the build.
    fn away() -> UseProbe {
        UseProbe { dir: [0.0, 1.0, 0.0], ..Self::above(0, 0) }
    }

    fn stone_in_hand() -> Vec<InventorySlot<'static>> {
        vec![InventorySlot { item: Self::STONE, count: 1 }]
    }

    fn closed_in() -> SpotWorld {
        SpotWorld { against: [true; 6] }
    }

    // Ticks until the aim at `probe` names a spot, and returns it.
    fn aim_when_ready(controller: &mut StratumController, probe: &UseProbe) -> [i32; 3] {
        let start = std::time::Instant::now();
        loop {
            controller.tick(&Frames::in_world(GhostScene::WORLD, 2.0));
            if let Some(spot) = controller.use_aim(probe) {
                return spot;
            }
            assert!(start.elapsed() < Self::DEADLINE, "the ghost never gave a spot");
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    // One full use call on the spot under `probe`, with every neighbour solid.
    fn place(controller: &mut StratumController, probe: &UseProbe) -> UseCommand {
        let spot = controller.use_aim(probe);
        let world = spot.map(|_| Self::closed_in());
        controller.use_decide(probe, world.as_ref(), &Self::stone_in_hand(), 0)
    }
}

#[test]
fn easy_place_off_is_vanilla() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = UseScene::start(data.path());
    controller.tick(&Frames::in_world(GhostScene::WORLD, 2.0));
    let probe = UseScene::above(0, 0);

    assert_eq!(controller.use_aim(&probe), None);
    assert_eq!(controller.use_decide(&probe, None, &UseScene::stone_in_hand(), 0), UseCommand::Vanilla);

    UseScene::enable(&mut controller);
    assert_eq!(UseScene::aim_when_ready(&mut controller, &probe), [0, 64, 0]);
}

#[test]
fn a_hidden_ghost_is_vanilla() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = UseScene::start(data.path());
    UseScene::enable(&mut controller);
    let probe = UseScene::above(1, 0);
    assert_eq!(UseScene::aim_when_ready(&mut controller, &probe), [1, 64, 0]);

    controller.apply_hud(HudAction::ToggleShown);
    controller.tick(&Frames::in_world(GhostScene::WORLD, 2.5));
    assert_eq!(controller.use_aim(&probe), None);
    assert_eq!(controller.use_decide(&probe, None, &UseScene::stone_in_hand(), 0), UseCommand::Vanilla);

    controller.apply_hud(HudAction::ToggleShown);
    controller.set_ghost_allowed(false);
    controller.tick(&Frames::in_world(GhostScene::WORLD, 3.0));
    assert_eq!(controller.use_aim(&probe), None);
    assert_eq!(controller.use_decide(&probe, None, &UseScene::stone_in_hand(), 0), UseCommand::Vanilla);
}

#[test]
fn a_turned_build_targets_the_turned_cell() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = UseScene::start(data.path());
    UseScene::enable(&mut controller);
    let (corner, turned) = Rotation::R90.turn_box([0; 3], [3, 1, 1]);
    let cell = [corner[0] as i32, 64, (corner[2] + i64::from(turned[2]) - 1) as i32];
    assert_ne!(cell[2], 0, "the far cell of the turned row is off the unturned row");
    let probe = UseScene::above(cell[0], cell[2]);
    controller.tick(&Frames::in_world(GhostScene::WORLD, 2.0));
    assert_eq!(controller.use_aim(&probe), None, "nothing stands there before the turn");

    controller.apply_hud(HudAction::Rotate(1));

    assert_eq!(UseScene::aim_when_ready(&mut controller, &probe), cell);
}

#[test]
fn a_screen_opening_ends_the_hold() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = UseScene::start(data.path());
    UseScene::enable(&mut controller);
    let probe = UseScene::above(0, 0);
    UseScene::aim_when_ready(&mut controller, &probe);
    let command = UseScene::place(&mut controller, &probe);
    assert_ne!(command, UseCommand::Vanilla);
    assert_eq!(controller.use_aim(&UseScene::away()), None);
    let held = controller.use_decide(&UseScene::away(), None, &UseScene::stone_in_hand(), 0);
    assert_eq!(held, UseCommand::Block, "a hold in progress blocks the vanilla use");

    let screen = FrameStatus { menu_open: true, ..Frames::in_world(GhostScene::WORLD, 3.0) };
    controller.tick(&screen);

    assert_eq!(controller.use_aim(&UseScene::away()), None);
    let after = controller.use_decide(&UseScene::away(), None, &UseScene::stone_in_hand(), 0);
    assert_eq!(after, UseCommand::Vanilla);
}

#[test]
fn use_released_ends_the_hold() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = UseScene::start(data.path());
    UseScene::enable(&mut controller);
    let probe = UseScene::above(0, 0);
    UseScene::aim_when_ready(&mut controller, &probe);
    assert_ne!(UseScene::place(&mut controller, &probe), UseCommand::Vanilla);
    assert_eq!(controller.easy_place_telemetry().placed, 1);

    controller.use_released();

    assert_eq!(controller.use_aim(&UseScene::away()), None);
    assert_eq!(controller.use_decide(&UseScene::away(), None, &UseScene::stone_in_hand(), 0), UseCommand::Vanilla);
}

#[test]
fn up_and_down_toggle_easy_place_and_save_it() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = RowScene::structure_marker(data.path());
    controller.hud_key(HudKey::Mode);
    while controller.hud_state().control != HudControl::EasyPlace {
        controller.hud_key(HudKey::Right);
    }

    controller.hud_key(HudKey::Up);
    controller.tick(&Frames::in_world("W1", 1.2));
    assert!(controller.snapshot().hud.clone().unwrap().easy_place);
    let marker = HudScene::saved(controller, data.path());
    assert!(marker.view.easy_place);

    let mut controller = ControllerHarness::start(data.path());
    controller.tick(&Frames::in_world("W1", 2.0));
    assert!(controller.snapshot().hud.clone().unwrap().easy_place, "the switch is read back from the saved marker");
    controller.hud_key(HudKey::Mode);
    while controller.hud_state().control != HudControl::EasyPlace {
        controller.hud_key(HudKey::Right);
    }
    controller.hud_key(HudKey::Down);
    controller.tick(&Frames::in_world("W1", 2.2));
    assert!(!controller.snapshot().hud.clone().unwrap().easy_place);
    let marker = HudScene::saved(controller, data.path());
    assert!(!marker.view.easy_place);
}

#[test]
fn the_hud_row_shows_easy_place_and_the_notice_for_three_seconds() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = UseScene::start(data.path());
    let hud = controller.snapshot().hud.clone().unwrap();
    assert_eq!((hud.easy_place, hud.notice), (false, None));
    UseScene::enable(&mut controller);
    assert!(controller.snapshot().hud.clone().unwrap().easy_place);
    let probe = UseScene::above(0, 0);
    UseScene::aim_when_ready(&mut controller, &probe);

    let walled_off = SpotWorld { against: [false; 6] };
    let command = controller.use_decide(&probe, Some(&walled_off), &UseScene::stone_in_hand(), 0);
    assert_eq!(command, UseCommand::Block);

    controller.tick(&Frames::in_world(GhostScene::WORLD, 2.0));
    assert_eq!(controller.snapshot().hud.clone().unwrap().notice, Some(EasyPlaceNotice::NoNeighbour));
    controller.tick(&Frames::in_world(GhostScene::WORLD, 4.9));
    assert_eq!(controller.snapshot().hud.clone().unwrap().notice, Some(EasyPlaceNotice::NoNeighbour));
    controller.tick(&Frames::in_world(GhostScene::WORLD, 2.0 + EasyPlaceService::NOTICE_SECONDS));
    assert_eq!(controller.snapshot().hud.clone().unwrap().notice, None);
}

#[test]
fn a_missing_block_notice_carries_the_needed_item_until_it_expires() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = UseScene::start(data.path());
    UseScene::enable(&mut controller);
    let probe = UseScene::above(0, 0);
    UseScene::aim_when_ready(&mut controller, &probe);

    let command = controller.use_decide(&probe, Some(&UseScene::closed_in()), &[], 0);
    assert_eq!(command, UseCommand::Block);

    controller.tick(&Frames::in_world(GhostScene::WORLD, 2.0));
    let hud = controller.snapshot().hud.clone().unwrap();
    assert_eq!(hud.notice, Some(EasyPlaceNotice::Missing { palette_id: 0 }));
    assert_eq!(hud.notice_item, Some(ItemId("minecraft:stone".to_owned())));
    controller.tick(&Frames::in_world(GhostScene::WORLD, 2.0 + EasyPlaceService::NOTICE_SECONDS));
    let hud = controller.snapshot().hud.clone().unwrap();
    assert_eq!((hud.notice, hud.notice_item), (None, None));
}

#[test]
fn a_pending_placement_expires_on_the_tick_without_a_use_call() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = UseScene::start(data.path());
    UseScene::enable(&mut controller);
    let probe = UseScene::above(0, 0);
    UseScene::aim_when_ready(&mut controller, &probe);
    assert!(matches!(UseScene::place(&mut controller, &probe), UseCommand::Place(_)));

    controller.tick(&Frames::in_world(GhostScene::WORLD, 4.0));

    assert_eq!(controller.easy_place_telemetry().rejected, 1);
}

#[test]
fn two_place_actions_for_one_target_give_one_marker() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    Scene::import(&mut controller, data.path(), "tower", &Scene::stone());
    controller.tick(&Frames::at("W1", [5, 64, -9], 1.0));
    // two actions from one frame, and one more from the next frame
    Scene::place(&mut controller, "Home");
    Scene::place(&mut controller, "Home");
    controller.tick(&Frames::at("W1", [5, 64, -9], 1.02));
    Scene::place(&mut controller, "Home");
    assert_eq!(controller.snapshot().markers.len(), 1);
    // the second click of a double click
    controller.tick(&Frames::at("W1", [5, 64, -9], 1.3));
    Scene::place(&mut controller, "Home");
    assert_eq!(controller.snapshot().markers.len(), 1);
    assert!(controller.snapshot().markers[0].active);

    // a click a moment later is a new marker, even from the same block
    controller.tick(&Frames::at("W1", [5, 64, -9], 2.0));
    Scene::place(&mut controller, "Home");
    assert_eq!(controller.snapshot().markers.len(), 2);
}

#[test]
fn a_place_for_another_target_at_once_is_a_new_marker() {
    let data = tempfile::tempdir().unwrap();
    let mut controller = ControllerHarness::start(data.path());
    Scene::import(&mut controller, data.path(), "tower", &Scene::stone());
    controller.tick(&Frames::at("W1", [5, 64, -9], 1.0));
    Scene::place(&mut controller, "Home");
    let structure = Scene::marker_target(&controller);
    controller.apply(LibraryAction::CreateProject("Keep".to_owned()));
    let project = MarkerTarget::Project(controller.snapshot().projects[0].id);
    controller.apply(LibraryAction::PlaceMarker { name: "Keep".to_owned(), target: project });
    let targets: Vec<MarkerTarget> = controller.snapshot().markers.iter().map(|row| row.target).collect();
    assert_eq!(targets.len(), 2);
    assert!(targets.contains(&structure) && targets.contains(&project));
}
