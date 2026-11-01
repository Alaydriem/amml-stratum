use std::io::Write;
use std::path::PathBuf;

use stratum_core::errors::StructureError;
use stratum_core::services::InboxScanner;
use stratum_core::structs::import::ImportFailure;

use crate::support::ImportHarness;

#[test]
fn inbox_ignores_other_extensions() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["a.mcstructure", "B.MCSTRUCTURE", "c.nbt", "d.mcstructure.txt", "mcstructure"] {
        ImportHarness::write(dir.path(), name, b"x");
    }
    std::fs::create_dir(dir.path().join("e.mcstructure")).unwrap();

    let scanner = InboxScanner::new(dir.path().to_path_buf());
    let mut found = Vec::new();
    scanner.scan(&mut found);
    scanner.scan(&mut found);
    assert_eq!(ImportHarness::file_names(&found), ["B.MCSTRUCTURE", "a.mcstructure"]);
}

#[test]
fn inbox_waits_for_a_file_to_stop_growing() {
    let dir = tempfile::tempdir().unwrap();
    let path = ImportHarness::write(dir.path(), "slow.mcstructure", b"first");
    let scanner = InboxScanner::new(dir.path().to_path_buf());
    let mut found: Vec<PathBuf> = Vec::new();

    scanner.scan(&mut found);
    assert!(found.is_empty());

    std::fs::OpenOptions::new().append(true).open(&path).unwrap().write_all(b" more").unwrap();
    scanner.scan(&mut found);
    assert!(found.is_empty());

    scanner.scan(&mut found);
    assert_eq!(ImportHarness::file_names(&found), ["slow.mcstructure"]);
}

#[test]
fn inbox_reports_a_file_once_while_it_stays() {
    let dir = tempfile::tempdir().unwrap();
    ImportHarness::write(dir.path(), "a.mcstructure", b"x");
    let scanner = InboxScanner::new(dir.path().to_path_buf());
    let mut found = Vec::new();
    for _ in 0..4 {
        scanner.scan(&mut found);
    }
    assert_eq!(found.len(), 1);
}

#[test]
fn inbox_scan_is_capped() {
    let dir = tempfile::tempdir().unwrap();
    for i in 0..300 {
        ImportHarness::write(dir.path(), &format!("s{i}.mcstructure"), b"x");
    }
    let scanner = InboxScanner::new(dir.path().to_path_buf());
    let mut found = Vec::new();
    scanner.scan(&mut found);
    scanner.scan(&mut found);
    assert_eq!(found.len(), 256);
}

#[test]
fn a_path_refused_as_busy_is_listed_again_after_release() {
    let inbox = tempfile::tempdir().unwrap();
    let path = ImportHarness::write(inbox.path(), "a.mcstructure", b"x");
    let scanner = InboxScanner::new(inbox.path().to_path_buf());

    let mut found = Vec::new();
    scanner.scan(&mut found);
    scanner.scan(&mut found);
    assert_eq!(found, std::slice::from_ref(&path));
    scanner.scan(&mut found);
    assert_eq!(found.len(), 1);

    scanner.release(&path);
    scanner.scan(&mut found);
    assert_eq!(found, [path.clone(), path]);
}

#[test]
fn bad_files_are_set_aside_under_unique_names() {
    let dir = tempfile::tempdir().unwrap();
    let scanner = InboxScanner::new(dir.path().to_path_buf());
    let reasons = [ImportFailure::NotStructure(StructureError::BadSize), ImportFailure::TooBig { bytes: 9 }];
    let mut found = Vec::new();
    for (round, reason) in reasons.iter().enumerate() {
        let path = ImportHarness::write(dir.path(), "bad.mcstructure", b"x");
        scanner.scan(&mut found);
        scanner.scan(&mut found);
        assert_eq!(found.len(), round + 1);
        scanner.reject(&path, reason);
        assert!(!path.exists());
    }
    assert!(dir.path().join("bad.mcstructure.failed").exists());
    assert!(dir.path().join("bad.mcstructure.failed-2").exists());
}

#[test]
fn an_unreadable_file_stays_and_is_not_listed_again() {
    let dir = tempfile::tempdir().unwrap();
    let path = ImportHarness::write(dir.path(), "locked.mcstructure", b"x");
    let scanner = InboxScanner::new(dir.path().to_path_buf());
    let mut found = Vec::new();
    scanner.scan(&mut found);
    scanner.scan(&mut found);
    scanner.reject(&path, &ImportFailure::Unreadable);
    found.clear();
    scanner.scan(&mut found);
    scanner.scan(&mut found);
    assert!(path.exists());
    assert!(found.is_empty());
}

#[test]
fn a_missing_inbox_lists_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let scanner = InboxScanner::new(dir.path().join("nope"));
    let mut found = Vec::new();
    scanner.scan(&mut found);
    assert!(found.is_empty());
}
