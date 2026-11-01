use stratum_core::config::LibraryConfig;
use stratum_core::services::{LibraryService, LibraryStore};
use stratum_core::structs::library::{Library, ProjectId, ProjectKind, StructureId};

use crate::support::NewStructureFixture;

#[test]
fn store_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let mut svc = LibraryService::new(Library::new(), LibraryConfig::default());
    let project = svc.create_project("Castle").unwrap();
    svc.set_project_kind(project, ProjectKind::BigBuild).unwrap();
    let mut new = NewStructureFixture::named("Tower");
    new.project = Some(project);
    new.world_origin = Some([-10, 64, 200]);
    svc.add_structure(new).unwrap();
    svc.add_structure(NewStructureFixture::named("Loose")).unwrap();

    let store = LibraryStore::new(dir.path().to_path_buf());
    store.save(svc.library()).unwrap();
    assert_eq!(&store.load().unwrap(), svc.library());
}

#[test]
fn store_missing_file_is_empty() {
    let dir = tempfile::tempdir().unwrap();
    let store = LibraryStore::new(dir.path().to_path_buf());
    assert_eq!(store.load().unwrap(), Library::new());
}

#[test]
fn store_corrupt_file_is_kept_and_load_is_empty() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("library.hcl"), "this is { not valid ===").unwrap();
    let store = LibraryStore::new(dir.path().to_path_buf());
    assert_eq!(store.load().unwrap(), Library::new());

    let names: Vec<String> =
        std::fs::read_dir(dir.path()).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    assert_eq!(names.len(), 1);
    assert!(names[0].starts_with("library.hcl.bad-"));
    let seconds = names[0].trim_start_matches("library.hcl.bad-");
    assert!(seconds.parse::<u64>().is_ok());
}

#[test]
fn store_clamps_missing_counters_above_existing_ids() {
    let dir = tempfile::tempdir().unwrap();
    let mut svc = LibraryService::new(Library::new(), LibraryConfig::default());
    for name in ["a", "b", "c"] {
        svc.add_structure(NewStructureFixture::named(name)).unwrap();
    }
    svc.create_project("p").unwrap();
    let store = LibraryStore::new(dir.path().to_path_buf());
    store.save(svc.library()).unwrap();

    let path = dir.path().join("library.hcl");
    let text = std::fs::read_to_string(&path).unwrap();
    let stripped: Vec<&str> = text.lines().filter(|line| !line.trim_start().starts_with("next_")).collect();
    assert!(stripped.len() < text.lines().count());
    std::fs::write(&path, stripped.join("\n")).unwrap();

    let mut loaded = LibraryService::new(store.load().unwrap(), LibraryConfig::default());
    assert_eq!(loaded.library().structures.len(), 3);
    assert_eq!(loaded.add_structure(NewStructureFixture::named("d")).unwrap(), StructureId(4));
    assert_eq!(loaded.create_project("q").unwrap(), ProjectId(2));
}

#[test]
fn store_non_utf8_file_is_kept_and_load_is_empty() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("library.hcl"), [0xff, 0xfe, 0x00, 0x80]).unwrap();
    let store = LibraryStore::new(dir.path().to_path_buf());
    assert_eq!(store.load().unwrap(), Library::new());

    let names: Vec<String> =
        std::fs::read_dir(dir.path()).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    assert_eq!(names.len(), 1);
    assert!(names[0].starts_with("library.hcl.bad-"));
}
