use stratum_core::config::ImportConfig;
use stratum_core::errors::{ImportBusy, LibraryError, StructureError};
use stratum_core::services::{ImportWorker, InboxScanner};
use stratum_core::structs::import::{ImportFailure, ImportResult};
use stratum_core::structs::structure::StructureSize;

use crate::support::{ImportHarness, ServiceFactory, StructureFixture};

#[test]
fn imports_a_valid_file() {
    let store = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    let bytes = StructureFixture::new([2, 3, 4])
        .block(0, 0, 0, "minecraft:stone")
        .block(1, 2, 3, "minecraft:dirt")
        .origin([5, 64, -7])
        .bytes();
    let source = ImportHarness::write(src.path(), "Tower.mcstructure", &bytes);

    let worker = ImportWorker::new(store.path().to_path_buf(), ImportConfig::default());
    worker.submit(ImportHarness::request(source.clone())).unwrap();
    let results = ImportHarness::collect(&worker, 1);

    let ImportResult::Ready { source: reported, stored_file, structure } = &results[0] else {
        panic!("expected Ready, got {:?}", results[0]);
    };
    assert_eq!(reported, &source);
    assert_eq!(stored_file, "Tower.mcstructure");
    assert_eq!(structure.display_name, "Tower");
    assert_eq!(&structure.file_name, stored_file);
    assert_eq!(structure.size, StructureSize::new(2, 3, 4));
    assert_eq!(structure.block_count, 2);
    assert_eq!(structure.world_origin, Some([5, 64, -7]));
    assert_eq!(std::fs::read(store.path().join("structures").join(stored_file)).unwrap(), bytes);
    assert!(source.exists());
}

#[test]
fn reports_a_corrupt_file_without_storing_it() {
    let store = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    let source = ImportHarness::write(src.path(), "bad.mcstructure", b"this is not nbt");

    let worker = ImportWorker::new(store.path().to_path_buf(), ImportConfig::default());
    worker.submit(ImportHarness::request(source.clone())).unwrap();
    let results = ImportHarness::collect(&worker, 1);

    assert!(matches!(&results[0], ImportResult::Failed { reason: ImportFailure::NotStructure(_), .. }));
    assert!(ImportHarness::stored_files(store.path()).is_empty());
    assert!(source.exists());
}

#[test]
fn refuses_files_over_the_byte_limit() {
    let store = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    let source = ImportHarness::write(src.path(), "big.mcstructure", &[0u8; 100]);

    let config = ImportConfig { max_file_bytes: 16, ..ImportConfig::default() };
    let worker = ImportWorker::new(store.path().to_path_buf(), config);
    worker.submit(ImportHarness::request(source)).unwrap();
    let results = ImportHarness::collect(&worker, 1);

    assert!(matches!(&results[0], ImportResult::Failed { reason: ImportFailure::TooBig { bytes: 100 }, .. }));
    assert!(ImportHarness::stored_files(store.path()).is_empty());
}

#[test]
fn a_missing_file_is_unreadable() {
    let store = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    let worker = ImportWorker::new(store.path().to_path_buf(), ImportConfig::default());
    worker.submit(ImportHarness::request(src.path().join("gone.mcstructure"))).unwrap();
    let results = ImportHarness::collect(&worker, 1);
    assert!(matches!(&results[0], ImportResult::Failed { reason: ImportFailure::Unreadable, .. }));
}

#[test]
fn a_structure_past_the_volume_limit_is_not_a_structure() {
    let store = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    let bytes = StructureFixture::new([1000, 1000, 1000]).bytes();
    let source = ImportHarness::write(src.path(), "huge.mcstructure", &bytes);

    let worker = ImportWorker::new(store.path().to_path_buf(), ImportConfig::default());
    worker.submit(ImportHarness::request(source)).unwrap();
    let results = ImportHarness::collect(&worker, 1);
    assert!(matches!(
        &results[0],
        ImportResult::Failed { reason: ImportFailure::NotStructure(StructureError::TooLarge { .. }), .. }
    ));
}

#[test]
fn import_runs_off_the_calling_thread() {
    let store = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    let source = ImportHarness::write(src.path(), "wall.mcstructure", &StructureFixture::full([64, 384, 64]).bytes());

    let worker = ImportWorker::new(store.path().to_path_buf(), ImportConfig::default());
    worker.pause();
    worker.submit(ImportHarness::request(source)).unwrap();
    let mut out = Vec::new();
    worker.drain(&mut out);
    assert!(out.is_empty());
    assert!(ImportHarness::stored_files(store.path()).is_empty());

    worker.resume();
    let results = ImportHarness::collect(&worker, 1);
    assert!(matches!(&results[0], ImportResult::Ready { .. }));
}

#[test]
fn vanilla_max_structure_imports_with_the_default_config() {
    let store = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    let source = ImportHarness::write(src.path(), "max.mcstructure", &StructureFixture::full([64, 384, 64]).bytes());

    let worker = ImportWorker::new(store.path().to_path_buf(), ImportConfig::default());
    worker.submit(ImportHarness::request(source)).unwrap();
    let results = ImportHarness::collect(&worker, 1);

    let ImportResult::Ready { structure, .. } = &results[0] else {
        panic!("expected Ready, got {:?}", results[0]);
    };
    assert_eq!(structure.size, StructureSize::new(64, 384, 64));
    assert_eq!(structure.block_count, 64 * 384 * 64);
}

#[test]
fn queue_full_returns_busy() {
    let store = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    let bytes = StructureFixture::new([1, 1, 1]).block(0, 0, 0, "minecraft:stone").bytes();
    let source = ImportHarness::write(src.path(), "one.mcstructure", &bytes);

    let config = ImportConfig { queue: 1, ..ImportConfig::default() };
    let worker = ImportWorker::new(store.path().to_path_buf(), config);
    worker.pause();
    // The queue holds one request and the paused worker may already hold one more.
    let mut accepted = 0;
    let mut busy = false;
    for _ in 0..3 {
        match worker.submit(ImportHarness::request(source.clone())) {
            Ok(()) => accepted += 1,
            Err(ImportBusy) => {
                busy = true;
                break;
            }
        }
    }
    assert!(busy);
    assert!((1..=2).contains(&accepted));

    worker.resume();
    assert_eq!(ImportHarness::collect(&worker, accepted).len(), accepted);
}

#[test]
fn stored_file_names_never_collide() {
    let store = tempfile::tempdir().unwrap();
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let bytes = StructureFixture::new([1, 1, 1]).block(0, 0, 0, "minecraft:stone").bytes();
    let a = ImportHarness::write(first.path(), "Tower.mcstructure", &bytes);
    let b = ImportHarness::write(second.path(), "Tower.mcstructure", &bytes);

    let worker = ImportWorker::new(store.path().to_path_buf(), ImportConfig::default());
    worker.submit(ImportHarness::request(a)).unwrap();
    worker.submit(ImportHarness::request(b)).unwrap();
    let results = ImportHarness::collect(&worker, 2);

    assert_eq!(ImportHarness::stored_files(store.path()), ["Tower-2.mcstructure", "Tower.mcstructure"]);

    // The library tells the two apart by display name.
    let mut library = ServiceFactory::with_capacity(10, 10);
    for result in results {
        let ImportResult::Ready { structure, .. } = result else { panic!("expected Ready") };
        library.add_structure(structure).unwrap();
    }
    let names: Vec<&str> = library.structures_in(None).iter().map(|e| e.display_name.as_str()).collect();
    assert_eq!(names, ["Tower", "Tower (2)"]);
}

#[test]
fn unicode_file_names_round_trip() {
    let store_root = tempfile::tempdir().unwrap();
    let src_root = tempfile::tempdir().unwrap();
    let store = store_root.path().join("Zoë Ann");
    let src = src_root.path().join("Zoë Ann");
    std::fs::create_dir_all(&src).unwrap();
    let bytes = StructureFixture::new([1, 1, 1]).block(0, 0, 0, "minecraft:stone").bytes();
    let source = ImportHarness::write(&src, "Hôtel de ville.mcstructure", &bytes);

    let worker = ImportWorker::new(store.clone(), ImportConfig::default());
    worker.submit(ImportHarness::request(source)).unwrap();
    let results = ImportHarness::collect(&worker, 1);

    let ImportResult::Ready { stored_file, structure, .. } = &results[0] else { panic!("expected Ready") };
    assert_eq!(structure.display_name, "Hôtel de ville");
    assert_eq!(stored_file, "Hôtel de ville.mcstructure");
    assert!(store.join("structures").join(stored_file).exists());
}

#[test]
fn a_refused_entry_leaves_the_source_and_loses_the_stored_copy() {
    let store = tempfile::tempdir().unwrap();
    let inbox = tempfile::tempdir().unwrap();
    let bytes = StructureFixture::new([1, 1, 1]).block(0, 0, 0, "minecraft:stone").bytes();
    let source = ImportHarness::write(inbox.path(), "Tower.mcstructure", &bytes);

    let worker = ImportWorker::new(store.path().to_path_buf(), ImportConfig::default());
    worker.submit(ImportHarness::request(source.clone())).unwrap();
    let results = ImportHarness::collect(&worker, 1);
    let ImportResult::Ready { stored_file, structure, .. } = results.into_iter().next().unwrap() else {
        panic!("expected Ready")
    };
    assert_eq!(ImportHarness::stored_files(store.path()), std::slice::from_ref(&stored_file));

    let mut library = ServiceFactory::with_capacity(0, 10);
    assert_eq!(library.add_structure(structure), Err(LibraryError::Full { capacity: 0 }));
    worker.discard_stored(&stored_file).unwrap();

    assert!(source.exists());
    assert!(ImportHarness::stored_files(store.path()).is_empty());
    assert!(worker.discard_stored("../library.hcl").is_err());
}

#[test]
fn an_accepted_inbox_file_is_removed_by_the_caller_and_not_listed_again() {
    let store = tempfile::tempdir().unwrap();
    let inbox = tempfile::tempdir().unwrap();
    let bytes = StructureFixture::new([1, 1, 1]).block(0, 0, 0, "minecraft:stone").bytes();
    let source = ImportHarness::write(inbox.path(), "Tower.mcstructure", &bytes);
    let scanner = InboxScanner::new(inbox.path().to_path_buf());
    let worker = ImportWorker::new(store.path().to_path_buf(), ImportConfig::default());
    let mut library = ServiceFactory::with_capacity(10, 10);

    let mut found = Vec::new();
    scanner.scan(&mut found);
    scanner.scan(&mut found);
    assert_eq!(found, std::slice::from_ref(&source));
    worker.submit(ImportHarness::request(source.clone())).unwrap();
    let results = ImportHarness::collect(&worker, 1);
    let ImportResult::Ready { source: done, structure, .. } = results.into_iter().next().unwrap() else {
        panic!("expected Ready")
    };
    // The worker left the source in place until the library accepted the entry.
    assert!(source.exists());
    library.add_structure(structure).unwrap();
    std::fs::remove_file(&done).unwrap();
    scanner.finish(&done);

    found.clear();
    scanner.scan(&mut found);
    scanner.scan(&mut found);
    assert!(found.is_empty());
}

#[test]
fn drop_joins_the_thread() {
    let store = tempfile::tempdir().unwrap();
    let src = tempfile::tempdir().unwrap();
    let source = ImportHarness::write(src.path(), "one.mcstructure", &StructureFixture::new([1, 1, 1]).bytes());

    let worker = ImportWorker::new(store.path().to_path_buf(), ImportConfig::default());
    let alive = worker.liveness();
    assert!(alive.upgrade().is_some());
    // A paused worker with a request in hand must still stop.
    worker.pause();
    worker.submit(ImportHarness::request(source)).unwrap();
    drop(worker);
    assert!(alive.upgrade().is_none());
}
