use alaydriem_stratum::ghost::GroupTracker;
use alaydriem_stratum::types::GroupStep;
use stratum_core::structs::ghost::{GhostJob, GhostJobMode, SectionKey, SectionPart};

fn job(slot: u32, serial: u32, index: u32, mode: GhostJobMode) -> GhostJob {
    let mut job = GhostJob::new(SectionKey { x: 0, y: 0, z: 0, layer: None, part: SectionPart::MiniatureGroup });
    job.slot = slot;
    job.group_serial = serial;
    job.group_index = index;
    job.mode = mode;
    job
}

#[test]
fn with_no_open_group_only_index_zero_applies() {
    let mut tracker = GroupTracker::new();
    tracker.resize(4);

    assert_eq!(tracker.step(&job(1, 7, 0, GhostJobMode::Append)), GroupStep::Apply);
    assert_eq!(tracker.step(&job(1, 7, 1, GhostJobMode::Append)), GroupStep::Ignore);
}

#[test]
fn a_finished_group_answers_its_finish_again_with_its_bytes() {
    let mut tracker = GroupTracker::new();
    tracker.resize(4);

    tracker.finish(2, 9, 480);

    assert_eq!(tracker.step(&job(2, 9, 3, GhostJobMode::Finish)), GroupStep::RepeatFinish);
    assert_eq!(tracker.finished_bytes(2), 480);
    tracker.free(2);
    assert_eq!(tracker.step(&job(2, 9, 3, GhostJobMode::Finish)), GroupStep::Ignore);
    assert_eq!(tracker.finished_bytes(2), 0);
}

#[test]
fn a_cleared_tracker_forgets_every_finished_group() {
    let mut tracker = GroupTracker::new();
    tracker.resize(4);
    tracker.finish(0, 1, 10);
    tracker.finish(3, 2, 20);

    tracker.clear();

    assert_eq!(tracker.step(&job(0, 1, 1, GhostJobMode::Finish)), GroupStep::Ignore);
    assert_eq!(tracker.step(&job(3, 2, 1, GhostJobMode::Finish)), GroupStep::Ignore);
    assert!(!tracker.is_open());
}
