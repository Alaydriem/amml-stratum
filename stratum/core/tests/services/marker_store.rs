use stratum_core::config::MarkerConfig;
use stratum_core::services::{MarkerService, MarkerStore};
use stratum_core::structs::library::{ProjectId, StructureId};
use stratum_core::structs::marker::{MarkerId, MarkerSet, MarkerTarget, Rotation, ViewMode, WorldKey};

use crate::support::MarkerFixture;

#[test]
fn store_round_trip_keeps_view_settings() {
    let dir = tempfile::tempdir().unwrap();
    let mut svc = MarkerFixture::service(8);
    let a = svc.place("Home", [10, 64, -20], MarkerTarget::Structure(StructureId(3))).unwrap();
    let b = svc.place("Castle", [0, 70, 0], MarkerTarget::Project(ProjectId(2))).unwrap();
    svc.activate(b).unwrap();
    let mut changed = svc.set().clone();
    let marker = changed.markers.iter_mut().find(|m| m.id == a).unwrap();
    marker.view.shown = false;
    marker.view.opacity_step = 4;
    marker.view.layer = 9;
    marker.view.view_mode = ViewMode::Whole;
    marker.view.easy_place = true;
    marker.view.selected = 5;

    let store = MarkerStore::new(dir.path().to_path_buf());
    store.save(&changed).unwrap();
    assert_eq!(store.load(&MarkerFixture::world()).unwrap(), changed);
}

#[test]
fn negative_origins_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let mut svc = MarkerFixture::service(8);
    svc.place("Deep", [-1, -64, -30_000_000], MarkerTarget::Structure(StructureId(1))).unwrap();
    let store = MarkerStore::new(dir.path().to_path_buf());
    store.save(svc.set()).unwrap();
    let loaded = store.load(&MarkerFixture::world()).unwrap();
    assert_eq!(loaded.markers[0].origin, [-1, -64, -30_000_000]);
}

#[test]
fn worlds_and_dimensions_are_separate_files() {
    let dir = tempfile::tempdir().unwrap();
    let store = MarkerStore::new(dir.path().to_path_buf());
    let keys = [
        WorldKey { world: "One".to_string(), dimension: 0 },
        WorldKey { world: "One".to_string(), dimension: -1 },
        WorldKey { world: "Two".to_string(), dimension: 0 },
    ];
    for (index, key) in keys.iter().enumerate() {
        let mut svc = MarkerService::new(MarkerSet::new(key.clone()), MarkerConfig::default());
        svc.place(&format!("m{index}"), [index as i32, 0, 0], MarkerTarget::Structure(StructureId(1))).unwrap();
        store.save(svc.set()).unwrap();
    }
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 3);
    for (index, key) in keys.iter().enumerate() {
        let loaded = store.load(key).unwrap();
        assert_eq!(&loaded.world, key);
        assert_eq!(loaded.markers.len(), 1);
        assert_eq!(loaded.markers[0].name, format!("m{index}"));
    }
}

#[test]
fn store_missing_file_is_empty() {
    let dir = tempfile::tempdir().unwrap();
    let store = MarkerStore::new(dir.path().to_path_buf());
    assert_eq!(store.load(&MarkerFixture::world()).unwrap(), MarkerSet::new(MarkerFixture::world()));
}

#[test]
fn store_corrupt_file_is_kept_and_load_is_empty() {
    let dir = tempfile::tempdir().unwrap();
    let stem = MarkerFixture::world().file_stem();
    std::fs::write(dir.path().join(format!("{stem}.hcl")), "this is { not valid ===").unwrap();
    let store = MarkerStore::new(dir.path().to_path_buf());
    assert_eq!(store.load(&MarkerFixture::world()).unwrap(), MarkerSet::new(MarkerFixture::world()));

    let names: Vec<String> =
        std::fs::read_dir(dir.path()).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    assert_eq!(names.len(), 1);
    let prefix = format!("{stem}.hcl.bad-");
    assert!(names[0].starts_with(&prefix));
    assert!(names[0].trim_start_matches(&prefix).parse::<u64>().is_ok());
}

#[test]
fn store_non_utf8_file_is_kept_and_load_is_empty() {
    let dir = tempfile::tempdir().unwrap();
    let stem = MarkerFixture::world().file_stem();
    std::fs::write(dir.path().join(format!("{stem}.hcl")), [0xff, 0xfe, 0x00, 0x80]).unwrap();
    let store = MarkerStore::new(dir.path().to_path_buf());
    assert_eq!(store.load(&MarkerFixture::world()).unwrap(), MarkerSet::new(MarkerFixture::world()));
    let names: Vec<String> =
        std::fs::read_dir(dir.path()).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    assert_eq!(names.len(), 1);
    assert!(names[0].starts_with(&format!("{stem}.hcl.bad-")));
}

#[test]
fn store_clamps_missing_counter_above_existing_ids() {
    let dir = tempfile::tempdir().unwrap();
    let mut svc = MarkerFixture::service(8);
    for name in ["a", "b", "c"] {
        svc.place(name, [0, 0, 0], MarkerTarget::Structure(StructureId(1))).unwrap();
    }
    let store = MarkerStore::new(dir.path().to_path_buf());
    store.save(svc.set()).unwrap();

    let path = dir.path().join(format!("{}.hcl", MarkerFixture::world().file_stem()));
    let text = std::fs::read_to_string(&path).unwrap();
    let stripped: Vec<&str> = text.lines().filter(|line| !line.trim_start().starts_with("next_")).collect();
    assert!(stripped.len() < text.lines().count());
    std::fs::write(&path, stripped.join("\n")).unwrap();

    let mut loaded = MarkerService::new(store.load(&MarkerFixture::world()).unwrap(), MarkerConfig::default());
    assert_eq!(loaded.markers().len(), 3);
    let id = loaded.place("d", [0, 0, 0], MarkerTarget::Structure(StructureId(1))).unwrap();
    assert_eq!(id, MarkerId(4));
}

#[test]
fn store_drops_an_active_id_no_marker_has() {
    let dir = tempfile::tempdir().unwrap();
    let mut svc = MarkerFixture::service(8);
    let first = svc.place("a", [0, 0, 0], MarkerTarget::Structure(StructureId(1))).unwrap();
    let mut dangling = svc.set().clone();
    dangling.active = Some(MarkerId(9));
    let store = MarkerStore::new(dir.path().to_path_buf());
    store.save(&dangling).unwrap();

    let loaded = store.load(&MarkerFixture::world()).unwrap();
    assert_eq!(loaded.active, None);
    assert_eq!(loaded.markers.len(), 1);
    assert_eq!(loaded.markers[0].id, first);
}

#[test]
fn phase_two_marker_view_is_clamped_on_load() {
    let dir = tempfile::tempdir().unwrap();
    let mut svc = MarkerFixture::service(8);
    svc.place("Old", [1, 2, 3], MarkerTarget::Structure(StructureId(1))).unwrap();
    let mut stored = svc.set().clone();
    stored.markers[0].view.opacity_step = 12;
    stored.markers[0].rotation = Rotation::R90;
    stored.markers[0].view.layer = 900;
    let store = MarkerStore::new(dir.path().to_path_buf());
    store.save(&stored).unwrap();

    let loaded = store.load(&MarkerFixture::world()).unwrap();

    let marker = &loaded.markers[0];
    assert_eq!(marker.view.opacity_step, 7);
    assert_eq!(marker.rotation, Rotation::R90);
    assert_eq!(marker.view.shown_layer(3), None);
    assert_eq!(marker.origin, [1, 2, 3]);
}
