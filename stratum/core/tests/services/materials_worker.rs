use std::time::{Duration, Instant};

use stratum_core::errors::StructureError;
use stratum_core::services::MaterialsWorker;
use stratum_core::structs::library::StructureId;
use stratum_core::structs::material::{StructureLoad, StructureLoaded};

use crate::support::{ControllerHarness, StructureFixture};

// Waits for `count` answers from the worker.
struct Answers;

impl Answers {
    fn collect(worker: &MaterialsWorker, count: usize) -> Vec<StructureLoaded> {
        let start = Instant::now();
        let mut found = Vec::new();
        while found.len() < count {
            assert!(start.elapsed() < Duration::from_secs(60), "loads did not arrive");
            worker.drain(&mut found);
            std::thread::yield_now();
        }
        found
    }
}

#[test]
fn a_stored_file_is_parsed_off_the_calling_thread() {
    let dir = tempfile::tempdir().unwrap();
    let good = ControllerHarness::picked_file(dir.path(), "a.mcstructure", &StructureFixture::full([2, 2, 2]).bytes());
    let bad = ControllerHarness::picked_file(dir.path(), "b.mcstructure", b"not nbt");
    let missing = dir.path().join("none.mcstructure");
    let worker = MaterialsWorker::new();
    for (number, path) in [good, bad, missing].into_iter().enumerate() {
        worker.submit(StructureLoad { id: StructureId(number as u64), path }).unwrap();
    }

    let answers = Answers::collect(&worker, 3);
    assert_eq!(answers[0].outcome.as_ref().unwrap().size.x, 2);
    assert!(matches!(answers[1].outcome, Err(StructureError::Nbt(_))));
    assert!(matches!(answers[2].outcome, Err(StructureError::Unreadable(_))));
}

#[test]
fn requests_past_the_queues_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = ControllerHarness::picked_file(dir.path(), "a.mcstructure", &StructureFixture::full([1, 1, 1]).bytes());
    let worker = MaterialsWorker::new();
    // Nothing is drained, so the worker stops once the result queue is full: the request
    // queue, the result queue and the one request in hand are all that is accepted.
    let accepted = (0..200u64)
        .filter(|number| worker.submit(StructureLoad { id: StructureId(*number), path: path.clone() }).is_ok())
        .count();
    assert!(accepted <= 2 * MaterialsWorker::QUEUE + 1);
    assert!(accepted >= MaterialsWorker::QUEUE);
}
