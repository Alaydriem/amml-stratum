use std::path::PathBuf;
use std::time::{Duration, Instant};

use stratum_core::config::ImportConfig;
use stratum_core::services::{ImportWorker, MaterialsWorker, StoreWriter};

// Pins that an idle worker thread waits on its queue and does not spin. A thread that waits with a
// 50 ms timeout wakes about 20 times a second.
struct IdleWakeups;

impl IdleWakeups {
    const SECONDS: f64 = 2.0;
    const LIMIT_PER_SECOND: f64 = 50.0;

    // Wake-ups a second over `SECONDS` of idle time, after the first pass of the thread loop.
    fn per_second(take: impl Fn() -> u64) -> f64 {
        std::thread::sleep(Duration::from_millis(100));
        let _ = take();
        let started = Instant::now();
        std::thread::sleep(Duration::from_secs_f64(Self::SECONDS));
        let rate = take() as f64 / started.elapsed().as_secs_f64();
        eprintln!("idle wake-ups per second: {rate:.1}");
        rate
    }
}

#[test]
fn the_idle_store_thread_does_not_spin() {
    let writer = StoreWriter::new();

    let rate = IdleWakeups::per_second(|| writer.take_wakeups());

    assert!(rate < IdleWakeups::LIMIT_PER_SECOND, "the store thread woke {rate} times a second while idle");
}

#[test]
fn the_idle_import_thread_does_not_spin() {
    let dir = tempfile::tempdir().unwrap();
    let worker = ImportWorker::new(PathBuf::from(dir.path()), ImportConfig::default());

    let rate = IdleWakeups::per_second(|| worker.take_wakeups());

    assert!(rate < IdleWakeups::LIMIT_PER_SECOND, "the import thread woke {rate} times a second while idle");
}

#[test]
fn the_idle_materials_thread_does_not_spin() {
    let worker = MaterialsWorker::new();

    let rate = IdleWakeups::per_second(|| worker.take_wakeups());

    assert!(rate < IdleWakeups::LIMIT_PER_SECOND, "the materials thread woke {rate} times a second while idle");
}
