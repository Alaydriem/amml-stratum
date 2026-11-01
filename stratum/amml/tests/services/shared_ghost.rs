use std::sync::Arc;

use alaydriem_stratum::services::{ReportQueue, SharedGhost};
use stratum_core::structs::ghost::{
    GhostDraw, GhostFrame, GhostJob, GhostPalette, GhostReport, SectionKey, SectionPart, SlotBuilt,
};
use stratum_core::structs::structure::{BlockState, BlockStates};

// Ghost frames as core would publish them, built by hand.
struct Frames;

impl Frames {
    const CAPACITY: u32 = 4;

    fn frame(generation: u64) -> GhostFrame {
        let mut frame = GhostFrame::new();
        frame.generation = generation;
        frame.slot_capacity = Self::CAPACITY;
        frame.opacity = 0.5;
        frame
    }

    fn job(slot: u32, serial: u32) -> GhostJob {
        let key = SectionKey { x: slot as i32, y: 0, z: 0, layer: None, part: SectionPart::Whole };
        let mut job = GhostJob::new(key);
        job.slot = slot;
        job.serial = serial;
        job.origin = [slot as i32 * 16, 0, 0];
        job.dims = [1, 1, 1];
        job.cells = vec![u16::MAX; 27];
        job.cells[13] = 0;
        job
    }

    fn palette() -> Arc<GhostPalette> {
        let stone = BlockState::new("minecraft:stone".to_string(), BlockStates::new(Vec::new()));
        Arc::new(GhostPalette { entries: vec![stone] })
    }

    // The (generation, slot, serial) of each job the next take hands out.
    fn taken_jobs(ghost: &SharedGhost) -> Vec<(u64, u32, u32)> {
        let take = ghost.take().unwrap();
        take.jobs().iter().map(|(generation, job)| (*generation, job.slot, job.serial)).collect()
    }
}

#[test]
fn jobs_are_handed_out_once_per_tick() {
    let ghost = SharedGhost::new();
    let mut frame = Frames::frame(1);
    frame.jobs.push(Frames::job(0, 1));
    frame.draws.push(GhostDraw { slot: 2, origin: [32, 0, 0], offset: [0; 3], scale: 1.0 });
    frame.palette = Some(Frames::palette());
    ghost.publish(&frame);

    {
        let first = ghost.take().unwrap();
        assert_eq!(first.jobs().len(), 1);
        assert_eq!(first.palette().map(|entries| entries.count()), Some(1));
    }
    let second = ghost.take().unwrap();

    assert!(second.jobs().is_empty());
    assert!(second.palette().is_none());
    assert_eq!(second.draws().len(), 1);
    assert_eq!((second.draws()[0].slot, second.draws()[0].origin), (2, [32, 0, 0]));
}

#[test]
fn the_next_tick_hands_out_only_its_own_jobs() {
    let ghost = SharedGhost::new();
    let mut first = Frames::frame(1);
    first.jobs.push(Frames::job(0, 1));
    ghost.publish(&first);
    drop(ghost.take());

    let mut second = Frames::frame(1);
    second.jobs.push(Frames::job(1, 2));
    ghost.publish(&second);

    assert_eq!(Frames::taken_jobs(&ghost), [(1, 1, 2)]);
}

#[test]
fn an_untaken_frame_latches_into_the_next() {
    let ghost = SharedGhost::new();
    let mut first = Frames::frame(1);
    first.clear_all = true;
    first.palette = Some(Frames::palette());
    first.jobs.push(Frames::job(0, 1));
    first.draws.push(GhostDraw { slot: 3, origin: [0, 0, 0], offset: [0; 3], scale: 1.0 });
    ghost.publish(&first);
    let mut second = Frames::frame(2);
    second.jobs.push(Frames::job(1, 2));
    ghost.publish(&second);

    let take = ghost.take().unwrap();

    assert!(take.clear_all());
    assert_eq!(take.generation(), 2);
    assert_eq!(take.palette().map(|entries| entries.count()), Some(1));
    let jobs: Vec<(u64, u32)> = take.jobs().iter().map(|(generation, job)| (*generation, job.slot)).collect();
    assert_eq!(jobs, [(1, 0), (2, 1)]);
    // draws are always the newest frame's
    assert!(take.draws().is_empty());
}

#[test]
fn a_latched_job_is_dropped_when_its_slot_is_freed_later() {
    let ghost = SharedGhost::new();
    let mut first = Frames::frame(1);
    first.jobs.push(Frames::job(0, 1));
    first.jobs.push(Frames::job(1, 2));
    ghost.publish(&first);
    let mut second = Frames::frame(1);
    second.frees.push(0);
    ghost.publish(&second);

    let take = ghost.take().unwrap();

    assert_eq!(take.frees(), [0]);
    let slots: Vec<u32> = take.jobs().iter().map(|(_, job)| job.slot).collect();
    assert_eq!(slots, [1]);
}

#[test]
fn a_freed_and_rebuilt_slot_keeps_its_free_and_the_new_job() {
    let ghost = SharedGhost::new();
    let mut first = Frames::frame(1);
    first.jobs.push(Frames::job(0, 1));
    ghost.publish(&first);
    let mut second = Frames::frame(1);
    second.frees.push(0);
    second.jobs.push(Frames::job(0, 2));
    ghost.publish(&second);

    let take = ghost.take().unwrap();

    assert_eq!(take.frees(), [0]);
    let jobs: Vec<(u32, u32)> = take.jobs().iter().map(|(_, job)| (job.slot, job.serial)).collect();
    assert_eq!(jobs, [(0, 2)]);
}

#[test]
fn a_later_clear_all_drops_the_latched_jobs_and_frees() {
    let ghost = SharedGhost::new();
    let mut first = Frames::frame(1);
    first.jobs.push(Frames::job(0, 1));
    first.frees.push(3);
    ghost.publish(&first);
    let mut second = Frames::frame(2);
    second.clear_all = true;
    second.jobs.push(Frames::job(1, 2));
    ghost.publish(&second);

    assert_eq!(Frames::taken_jobs(&ghost), [(2, 1, 2)]);
    let mut third = Frames::frame(2);
    third.frees.push(1);
    ghost.publish(&third);
    let take = ghost.take().unwrap();
    // the clear was taken with the second frame and is not repeated
    assert!(!take.clear_all());
    assert_eq!(take.frees(), [1]);
}

#[test]
fn a_new_job_for_a_latched_slot_replaces_it() {
    let ghost = SharedGhost::new();
    let mut first = Frames::frame(1);
    first.jobs.push(Frames::job(0, 1));
    ghost.publish(&first);
    let mut second = Frames::frame(1);
    second.jobs.push(Frames::job(0, 5));
    ghost.publish(&second);

    assert_eq!(Frames::taken_jobs(&ghost), [(1, 0, 5)]);
}

#[test]
fn the_newest_palette_is_kept_while_untaken() {
    let ghost = SharedGhost::new();
    let mut first = Frames::frame(1);
    first.palette = Some(Frames::palette());
    ghost.publish(&first);
    let mut second = Frames::frame(2);
    let mut palette = (*Frames::palette()).clone();
    palette.entries.push(BlockState::new("minecraft:dirt".to_string(), BlockStates::new(Vec::new())));
    second.palette = Some(Arc::new(palette));
    ghost.publish(&second);
    ghost.publish(&Frames::frame(2));

    let take = ghost.take().unwrap();

    let entries: Vec<Vec<u8>> = take.palette().unwrap().map(<[u8]>::to_vec).collect();
    assert_eq!(entries.len(), 2);
    assert!(entries[1].windows(b"minecraft:dirt".len()).any(|window| window == b"minecraft:dirt"));
}

#[test]
fn latched_jobs_stay_within_the_slot_capacity() {
    let ghost = SharedGhost::new();
    let mut serial = 0;
    for _ in 0..3 {
        for slot in 0..(Frames::CAPACITY * 2) {
            serial += 1;
            let mut frame = Frames::frame(1);
            frame.jobs.push(Frames::job(slot, serial));
            frame.frees.push(slot);
            ghost.publish(&frame);
        }
    }

    let take = ghost.take().unwrap();

    // one job per slot, the newest, and nothing for a slot past the capacity
    let jobs: Vec<(u32, u32)> = take.jobs().iter().map(|(_, job)| (job.slot, job.serial)).collect();
    let expected: Vec<(u32, u32)> =
        (0..Frames::CAPACITY).map(|slot| (slot, 2 * 2 * Frames::CAPACITY + slot + 1)).collect();
    assert_eq!(jobs, expected);
    assert_eq!(take.frees(), [0, 1, 2, 3]);
}

#[test]
fn a_taken_slot_takes_frees_and_jobs_again() {
    let ghost = SharedGhost::new();
    let mut first = Frames::frame(1);
    first.frees.push(2);
    first.jobs.push(Frames::job(2, 1));
    ghost.publish(&first);
    drop(ghost.take());

    let mut second = Frames::frame(1);
    second.frees.push(2);
    second.jobs.push(Frames::job(2, 2));
    ghost.publish(&second);

    let take = ghost.take().unwrap();
    assert_eq!(take.frees(), [2]);
    assert_eq!(take.jobs().len(), 1);
}

#[test]
fn the_world_epoch_reaches_the_take() {
    let ghost = SharedGhost::new();
    let mut frame = Frames::frame(1);
    frame.world_epoch = 3;
    frame.in_world = true;
    ghost.publish(&frame);

    let take = ghost.take().unwrap();

    assert_eq!(take.world_epoch(), 3);
    assert!(take.in_world());
}

#[test]
fn a_full_report_queue_folds_into_the_newest_and_loses_no_entry() {
    let ghost = SharedGhost::new();
    let mut frame = Frames::frame(1);
    frame.slot_capacity = 16;
    ghost.publish(&frame);
    let count = ReportQueue::CAPACITY as u32 + 3;
    for n in 1..=count {
        let queued = ghost.report(false, |report| {
            report.built.clear();
            report.built.push(SlotBuilt { slot: n, serial: n, generation: 1, bytes: 20, ok: true });
            report.draw_frames = 1;
            report.cells_built = 10;
            report.build_us = u64::from(n) * 100;
        });
        assert_eq!(queued, Some(()));
    }

    let mut slots = Vec::new();
    let mut frames = 0;
    let mut cells = 0;
    let mut newest_build_us = 0;
    ghost.drain(|report: &GhostReport| {
        slots.extend(report.built.iter().map(|built| built.slot));
        frames += report.draw_frames;
        cells += report.cells_built;
        newest_build_us = report.build_us;
    });

    assert_eq!(slots, (1..=count).collect::<Vec<_>>());
    assert_eq!((frames, cells), (count, count * 10));
    // the newest report holds the slowest of the frames folded into it
    assert_eq!(newest_build_us, u64::from(count) * 100);
    assert_eq!(ghost.merged_reports(), 3);
    assert_eq!(ghost.dropped_reports(), 0);
}

#[test]
fn folded_entries_stay_within_the_slot_capacity() {
    let ghost = SharedGhost::new();
    ghost.publish(&Frames::frame(1));
    for n in 0..(ReportQueue::CAPACITY as u32 + 10) {
        ghost.report(false, |report| {
            report.built.clear();
            report.built.push(SlotBuilt { slot: n % Frames::CAPACITY, serial: n, generation: 1, bytes: 20, ok: true });
        });
    }

    let mut newest = 0;
    ghost.drain(|report| newest = report.built.len());

    assert_eq!(newest, Frames::CAPACITY as usize);
}

#[test]
fn a_folded_palette_count_is_the_newest() {
    let ghost = SharedGhost::new();
    ghost.publish(&Frames::frame(1));
    for n in 0..(ReportQueue::CAPACITY as u32 + 2) {
        ghost.report(false, |report| {
            report.palette_resolved = n == ReportQueue::CAPACITY as u32;
            report.unknown_blocks = if report.palette_resolved { 7 } else { 0 };
        });
    }

    let mut counts = Vec::new();
    ghost.drain(|report| counts.push((report.palette_resolved, report.unknown_blocks)));

    assert_eq!(counts.last(), Some(&(true, 7)));
}

#[test]
fn a_drained_queue_takes_reports_again() {
    let ghost = SharedGhost::new();
    ghost.report(false, |report| report.unknown_blocks = 1);
    ghost.drain(|_| {});

    ghost.report(false, |report| report.unknown_blocks = 2);
    let mut seen = Vec::new();
    ghost.drain(|report| seen.push(report.unknown_blocks));

    assert_eq!(seen, [2]);
    assert_eq!(ghost.dropped_reports(), 0);
}

#[test]
fn report_entries_are_bounded_by_the_slot_capacity() {
    let ghost = SharedGhost::new();
    assert_eq!(ghost.built_limit(3), 0);

    ghost.publish(&Frames::frame(1));

    assert_eq!(ghost.built_limit(3), 3);
    assert_eq!(ghost.built_limit(u32::MAX), Frames::CAPACITY as usize);
}

#[test]
fn a_world_reset_reaches_the_tick_once_through_a_full_queue() {
    let ghost = SharedGhost::new();
    ghost.report(true, |report| report.world_reset = true);
    for _ in 0..ReportQueue::CAPACITY {
        ghost.report(false, |_| {});
    }
    assert_eq!(ghost.merged_reports(), 1);

    let mut resets = 0;
    ghost.drain(|report| resets += usize::from(report.world_reset));

    assert_eq!(resets, 1);
    let mut again = 0;
    ghost.drain(|report| again += usize::from(report.world_reset));
    assert_eq!(again, 0);
}

#[test]
fn a_world_reset_survives_a_busy_queue() {
    let ghost = SharedGhost::new();
    ghost.report(false, |_| {});
    let mut resets = 0;

    // the tick holds the queue while it applies, so this report is dropped
    ghost.drain(|report| {
        resets += usize::from(report.world_reset);
        if !report.world_reset {
            ghost.report(true, |report| report.world_reset = true);
        }
    });

    assert_eq!(ghost.dropped_reports(), 1);
    assert_eq!(resets, 1);
}

#[test]
fn entries_cut_by_a_fold_are_counted_as_dropped() {
    let ghost = SharedGhost::new();
    ghost.publish(&Frames::frame(1));
    let entry = |slot| SlotBuilt { slot, serial: 1, generation: 1, bytes: 20, ok: true };
    for _ in 0..ReportQueue::CAPACITY {
        ghost.report(false, |report| {
            report.built.clear();
            report.built.push(entry(0));
        });
    }
    assert_eq!(ghost.dropped_entries(), 0);

    // the newest report holds 1 entry, so 4 more leave room for 3 and cut 1
    ghost.report(false, |report| {
        report.built.clear();
        report.built.extend((0..Frames::CAPACITY).map(entry));
    });

    assert_eq!(ghost.merged_reports(), 1);
    assert_eq!(ghost.dropped_entries(), 1);
    assert_eq!(ghost.dropped_reports(), 0);
}

#[test]
fn a_fold_that_keeps_every_entry_drops_nothing() {
    let ghost = SharedGhost::new();
    ghost.publish(&Frames::frame(1));
    for _ in 0..(ReportQueue::CAPACITY + 2) {
        ghost.report(false, |report| report.built.clear());
    }

    assert_eq!(ghost.merged_reports(), 2);
    assert_eq!(ghost.dropped_reports(), 0);
    assert_eq!(ghost.dropped_entries(), 0);
}
