use std::time::Duration;

use stratum_core::services::StoreWriter;

const FLUSH: Duration = Duration::from_secs(60);

#[test]
fn a_save_request_returns_before_the_disk_is_touched() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("library.hcl");
    let writer = StoreWriter::new();
    writer.pause();

    writer.submit(path.clone(), b"state".to_vec()).unwrap();
    assert_eq!(writer.pending(), 1);
    assert!(!path.exists());

    writer.resume();
    assert!(writer.flush(FLUSH));
    assert_eq!(std::fs::read(&path).unwrap(), b"state");
    assert_eq!(writer.pending(), 0);
}

#[test]
fn the_newest_bytes_win_for_a_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("library.hcl");
    let writer = StoreWriter::new();
    writer.pause();
    for version in 1..=3 {
        writer.submit(path.clone(), format!("version {version}").into_bytes()).unwrap();
    }
    assert_eq!(writer.pending(), 1);

    writer.resume();
    assert!(writer.flush(FLUSH));
    assert_eq!(std::fs::read(&path).unwrap(), b"version 3");
}

#[test]
fn paths_past_the_bound_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let writer = StoreWriter::new();
    writer.pause();
    for number in 0..StoreWriter::MAX_PENDING_PATHS {
        writer.submit(dir.path().join(format!("w{number}.hcl")), b"x".to_vec()).unwrap();
    }
    let extra = dir.path().join("extra.hcl");
    assert!(writer.submit(extra.clone(), b"x".to_vec()).is_err());
    assert_eq!(writer.pending(), StoreWriter::MAX_PENDING_PATHS);

    // A path already waiting still takes newer bytes.
    writer.submit(dir.path().join("w0.hcl"), b"newer".to_vec()).unwrap();
    assert_eq!(writer.pending(), StoreWriter::MAX_PENDING_PATHS);

    writer.resume();
    assert!(writer.flush(FLUSH));
    assert_eq!(std::fs::read(dir.path().join("w0.hcl")).unwrap(), b"newer");
    assert!(!extra.exists());
}

#[test]
fn a_streak_of_failures_logs_once() {
    let dir = tempfile::tempdir().unwrap();
    let writer = StoreWriter::new();
    // A folder in the way of the temporary file makes the write fail.
    std::fs::create_dir_all(dir.path().join("bad.hcl.tmp")).unwrap();
    let bad = dir.path().join("bad.hcl");
    let good = dir.path().join("good.hcl");

    for _ in 0..3 {
        writer.submit(bad.clone(), b"x".to_vec()).unwrap();
        assert!(writer.flush(FLUSH));
    }
    assert_eq!(writer.failures(), 3);
    assert_eq!(writer.failure_logs(), 1);
    assert!(writer.failing());

    // A success ends the streak, so the next failure logs again.
    writer.submit(good, b"x".to_vec()).unwrap();
    assert!(writer.flush(FLUSH));
    assert!(!writer.failing());
    writer.submit(bad, b"x".to_vec()).unwrap();
    assert!(writer.flush(FLUSH));
    assert_eq!(writer.failures(), 4);
    assert_eq!(writer.failure_logs(), 2);
}

#[test]
fn failed_writes_are_kept_and_retried() {
    let dir = tempfile::tempdir().unwrap();
    let writer = StoreWriter::new();
    let blocker = dir.path().join("a.hcl.tmp");
    std::fs::create_dir_all(&blocker).unwrap();
    let path = dir.path().join("a.hcl");

    writer.submit(path.clone(), b"kept".to_vec()).unwrap();
    assert!(writer.flush(FLUSH));
    assert_eq!(writer.failed(), 1);
    assert!(!path.exists());

    std::fs::remove_dir_all(&blocker).unwrap();
    writer.retry_failed();
    assert!(writer.flush(FLUSH));
    assert_eq!(std::fs::read(&path).unwrap(), b"kept");
    assert_eq!(writer.failed(), 0);
}

#[test]
fn newer_bytes_replace_a_failed_write() {
    let dir = tempfile::tempdir().unwrap();
    let writer = StoreWriter::new();
    let blocker = dir.path().join("a.hcl.tmp");
    std::fs::create_dir_all(&blocker).unwrap();
    let path = dir.path().join("a.hcl");
    writer.submit(path.clone(), b"old".to_vec()).unwrap();
    assert!(writer.flush(FLUSH));
    std::fs::remove_dir_all(&blocker).unwrap();

    writer.submit(path.clone(), b"new".to_vec()).unwrap();
    assert_eq!(writer.failed(), 0);
    assert!(writer.flush(FLUSH));
    writer.retry_failed();
    assert!(writer.flush(FLUSH));
    assert_eq!(std::fs::read(&path).unwrap(), b"new");
}

#[test]
fn dropping_the_writer_writes_what_is_pending() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested").join("library.hcl");
    let writer = StoreWriter::new();
    writer.pause();
    writer.submit(path.clone(), b"last".to_vec()).unwrap();
    drop(writer);
    assert_eq!(std::fs::read(&path).unwrap(), b"last");
}

#[test]
fn close_stops_taking_files_whatever_is_waiting() {
    let dir = tempfile::tempdir().unwrap();
    let mut writer = StoreWriter::new();
    writer.pause();
    for number in 0..20 {
        writer.submit(dir.path().join(format!("w{number}.hcl")), b"x".to_vec()).unwrap();
    }

    // A plain drop would write all 20 files; close leaves them and returns within the wait.
    assert!(writer.close(FLUSH));
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    assert_eq!(writer.failure_logs(), 1);
    assert!(writer.submit(dir.path().join("late.hcl"), b"x".to_vec()).is_err());
}

#[test]
fn files_that_will_not_reach_the_disk_are_logged_at_close() {
    let dir = tempfile::tempdir().unwrap();
    let mut writer = StoreWriter::new();
    std::fs::create_dir_all(dir.path().join("a.hcl.tmp")).unwrap();
    writer.submit(dir.path().join("a.hcl"), b"x".to_vec()).unwrap();
    assert!(writer.flush(FLUSH));
    assert_eq!(writer.failure_logs(), 1);

    assert!(writer.close(FLUSH));
    assert_eq!(writer.failure_logs(), 2);
}

#[test]
fn a_clean_close_logs_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let mut writer = StoreWriter::new();
    writer.submit(dir.path().join("a.hcl"), b"x".to_vec()).unwrap();
    assert!(writer.flush(FLUSH));
    assert!(writer.close(FLUSH));
    assert_eq!(writer.failure_logs(), 0);
    assert_eq!(std::fs::read(dir.path().join("a.hcl")).unwrap(), b"x");
}

#[test]
fn a_retry_keeps_the_writes_that_do_not_fit() {
    let dir = tempfile::tempdir().unwrap();
    let writer = StoreWriter::new();
    let failing: Vec<_> = ["a", "b", "c"].iter().map(|name| dir.path().join(format!("{name}.hcl"))).collect();
    for path in &failing {
        std::fs::create_dir_all(format!("{}.tmp", path.display())).unwrap();
        writer.submit(path.clone(), b"kept".to_vec()).unwrap();
    }
    assert!(writer.flush(FLUSH));
    assert_eq!(writer.failed(), 3);
    for path in &failing {
        std::fs::remove_dir_all(format!("{}.tmp", path.display())).unwrap();
    }

    // Two places are left under the bound, so one failed write has to wait.
    writer.pause();
    for number in 0..StoreWriter::MAX_PENDING_PATHS - 2 {
        writer.submit(dir.path().join(format!("fill{number}.hcl")), b"x".to_vec()).unwrap();
    }
    writer.retry_failed();
    assert_eq!(writer.pending(), StoreWriter::MAX_PENDING_PATHS);
    assert_eq!(writer.failed(), 1);

    writer.resume();
    assert!(writer.flush(FLUSH));
    writer.retry_failed();
    assert!(writer.flush(FLUSH));
    assert_eq!(writer.failed(), 0);
    for path in &failing {
        assert_eq!(std::fs::read(path).unwrap(), b"kept");
    }
}

#[test]
fn a_path_is_saved_only_once_its_newest_bytes_are_on_disk() {
    let dir = tempfile::tempdir().unwrap();
    let writer = StoreWriter::new();
    let path = dir.path().join("a.hcl");
    writer.pause();
    writer.submit(path.clone(), b"one".to_vec()).unwrap();
    assert!(!writer.saved(&path), "waiting bytes are not saved");
    writer.resume();
    assert!(writer.flush(FLUSH));
    assert!(writer.saved(&path));

    let blocker = dir.path().join("a.hcl.tmp");
    std::fs::create_dir_all(&blocker).unwrap();
    writer.submit(path.clone(), b"two".to_vec()).unwrap();
    assert!(writer.flush(FLUSH));
    assert!(!writer.saved(&path), "a failed write is not saved");
    std::fs::remove_dir_all(&blocker).unwrap();
    writer.retry_failed();
    assert!(writer.flush(FLUSH));
    assert!(writer.saved(&path));
}

#[test]
fn a_dropped_failure_means_nothing_counts_as_saved() {
    let dir = tempfile::tempdir().unwrap();
    let writer = StoreWriter::new();
    let good = dir.path().join("good.hcl");
    writer.submit(good.clone(), b"x".to_vec()).unwrap();
    assert!(writer.flush(FLUSH));
    assert!(writer.saved(&good));
    // One more failing path than the failed list holds.
    for number in 0..=StoreWriter::MAX_PENDING_PATHS {
        std::fs::create_dir_all(dir.path().join(format!("w{number}.hcl.tmp"))).unwrap();
        writer.submit(dir.path().join(format!("w{number}.hcl")), b"x".to_vec()).unwrap();
        assert!(writer.flush(FLUSH));
    }
    assert_eq!(writer.failed(), StoreWriter::MAX_PENDING_PATHS);
    assert!(!writer.saved(&good));
}
