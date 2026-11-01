use stratum_core::services::{GhostScheduler, GhostSections};
use stratum_core::structs::ghost::{
    GhostDraw, GhostJob, GhostJobMode, GhostReport, MiniatureGroups, SectionKey, SectionPart, SlotBuilt,
};

use crate::support::{GhostFixture, SchedulerHarness};

#[test]
fn jobs_follow_the_wanted_order_up_to_the_cell_budget() {
    let build = GhostFixture::blocks(&[([0, 0, 0], [4, 5, 0]), ([16, 0, 0], [20, 5, 0]), ([32, 0, 0], [36, 5, 0])]);
    let wanted = SchedulerHarness::wanted(&build);
    let mut harness = SchedulerHarness::new(64, 64);

    harness.plan(&build, &wanted);

    assert_eq!(harness.job_keys(), wanted[..2]);
    assert_eq!(harness.frame.jobs[0].slot, 0);
    assert_eq!(harness.frame.jobs[1].slot, 1);
}

#[test]
fn at_least_one_job_per_frame_even_when_one_job_passes_the_budget() {
    let build = GhostFixture::blocks(&[([0, 0, 0], [9, 9, 1]), ([16, 0, 0], [16, 0, 0])]);
    let wanted = SchedulerHarness::wanted(&build);
    let mut harness = SchedulerHarness::new(64, 64);

    harness.plan(&build, &wanted);

    assert_eq!(harness.job_keys(), wanted[..1]);
}

#[test]
fn a_wanted_section_is_built_once() {
    let build = SchedulerHarness::row(1);
    let wanted = SchedulerHarness::wanted(&build);
    let mut harness = SchedulerHarness::new(64, 600);

    harness.plan(&build, &wanted);
    assert_eq!(harness.frame.jobs.len(), 1);
    assert!(harness.frame.draws.is_empty());
    harness.report_jobs(true, 1000);
    harness.plan(&build, &wanted);

    assert!(harness.frame.jobs.is_empty());
    assert_eq!(harness.draw_slots(), [0]);
    assert_eq!(harness.frame.draws[0].origin, [0, 0, 0]);
}

#[test]
fn unwanted_slots_are_freed() {
    let build = SchedulerHarness::row(2);
    let wanted = SchedulerHarness::wanted(&build);
    let mut harness = SchedulerHarness::new(64, 600);
    harness.plan(&build, &wanted);
    harness.report_jobs(true, 1000);

    harness.plan(&build, &wanted[..1]);

    assert_eq!(harness.frame.frees, [1]);
    assert_eq!(harness.draw_slots(), [0]);
    assert_eq!(harness.scheduler.live_slots(), 1);
}

#[test]
fn new_build_generation_clears_every_slot_and_drops_old_reports() {
    let first = SchedulerHarness::row(2);
    let mut second = first.clone();
    second.revision = 2;
    let wanted = SchedulerHarness::wanted(&first);
    let mut harness = SchedulerHarness::new(64, 600);
    harness.plan(&first, &wanted);
    assert!(harness.frame.palette.is_some());
    let old_generation = harness.frame.generation;
    let old_serial = harness.frame.jobs[0].serial;

    harness.plan(&second, &wanted);

    assert!(harness.frame.clear_all);
    assert_eq!(harness.frame.generation, 2);
    assert!(harness.frame.palette.is_some());
    assert_eq!(harness.frame.jobs.len(), 2);
    let mut stale = GhostReport::new();
    stale.built.push(SlotBuilt { slot: 0, serial: old_serial, generation: old_generation, bytes: 1000, ok: true });
    harness.scheduler.apply_report(&stale);
    harness.plan(&second, &wanted);

    assert!(!harness.frame.clear_all);
    assert!(harness.frame.palette.is_none());
    assert!(harness.frame.draws.is_empty());
    assert_eq!(harness.scheduler.used_bytes(), 0);
}

#[test]
fn no_build_clears_every_slot_and_hands_out_nothing() {
    let build = SchedulerHarness::row(2);
    let wanted = SchedulerHarness::wanted(&build);
    let mut harness = SchedulerHarness::new(64, 600);
    harness.plan(&build, &wanted);
    harness.report_jobs(true, 1000);

    harness.scheduler.plan(None, &wanted, &MiniatureGroups::new(), [0, 0, 0], &mut harness.frame);

    assert!(harness.frame.clear_all);
    assert!(harness.frame.jobs.is_empty());
    assert!(harness.frame.draws.is_empty());
    assert_eq!(harness.scheduler.live_slots(), 0);
}

#[test]
fn budget_full_evicts_the_farthest_wanted_section_and_sets_cut() {
    let build = SchedulerHarness::row(5);
    let wanted = SchedulerHarness::wanted(&build);
    let mut harness = SchedulerHarness::new(1, 600);
    assert_eq!(harness.scheduler.slot_capacity(), 4);

    harness.plan(&build, &wanted);
    assert_eq!(harness.job_keys(), wanted[..4]);
    assert!(harness.frame.cut);
    harness.report_jobs(true, 1000);
    harness.plan(&build, &wanted);
    assert!(harness.frame.jobs.is_empty());
    assert!(harness.frame.cut);
    assert_eq!(harness.draw_slots(), [0, 1, 2, 3]);

    let reversed = [wanted[4], wanted[3], wanted[2], wanted[1], wanted[0]];
    harness.plan(&build, &reversed);

    assert_eq!(harness.job_keys(), [wanted[4]]);
    assert_eq!(harness.frame.jobs[0].slot, 0);
    assert!(harness.frame.cut);
    assert_eq!(harness.draw_slots(), [3, 2, 1]);
}

#[test]
fn reported_bytes_count_against_the_budget() {
    let build = SchedulerHarness::row(3);
    let wanted = SchedulerHarness::wanted(&build);
    let mut harness = SchedulerHarness::new(2, 600);
    assert_eq!(harness.scheduler.slot_capacity(), 8);
    harness.plan(&build, &wanted[..2]);
    harness.report_jobs(true, 1024 * 1024);
    assert_eq!(harness.scheduler.used_bytes(), 2 * 1024 * 1024);

    harness.plan(&build, &wanted);

    assert!(harness.frame.jobs.is_empty());
    assert!(harness.frame.cut);
    assert_eq!(harness.scheduler.live_slots(), 2);
    assert_eq!(GhostScheduler::SLOT_BYTES_ESTIMATE, 256 * 1024);
}

#[test]
fn failed_build_is_not_retried_in_the_same_generation() {
    let build = SchedulerHarness::row(1);
    let wanted = SchedulerHarness::wanted(&build);
    let mut harness = SchedulerHarness::new(64, 600);
    harness.plan(&build, &wanted);
    harness.report_jobs(false, 0);

    harness.plan(&build, &wanted);
    assert!(harness.frame.jobs.is_empty());
    assert_eq!(harness.frame.frees, [0]);
    assert!(harness.frame.draws.is_empty());
    harness.plan(&build, &wanted);
    assert!(harness.frame.jobs.is_empty());

    let mut next = build.clone();
    next.revision = 2;
    harness.plan(&next, &wanted);
    assert_eq!(harness.frame.jobs.len(), 1);
}

#[test]
fn plan_reuses_frame_buffers() {
    let build = SchedulerHarness::row(2);
    let wanted = SchedulerHarness::wanted(&build);
    let mut harness = SchedulerHarness::new(64, 600);
    harness.plan(&build, &wanted);
    harness.report_jobs(true, 1000);
    harness.scheduler.invalidate(&wanted);
    harness.plan(&build, &wanted);
    harness.report_jobs(true, 1000);
    let before = (
        harness.frame.jobs.capacity(),
        harness.frame.draws.capacity(),
        harness.frame.frees.capacity(),
        harness.frame.jobs.iter().map(|job| job.cells.capacity()).collect::<Vec<_>>(),
    );

    harness.scheduler.invalidate(&wanted);
    harness.plan(&build, &wanted);

    assert_eq!(harness.frame.jobs.len(), 2);
    let after = (
        harness.frame.jobs.capacity(),
        harness.frame.draws.capacity(),
        harness.frame.frees.capacity(),
        harness.frame.jobs.iter().map(|job| job.cells.capacity()).collect::<Vec<_>>(),
    );
    assert_eq!(before, after);
}

#[test]
fn invalidated_slot_is_rebuilt_in_place() {
    let build = SchedulerHarness::row(1);
    let wanted = SchedulerHarness::wanted(&build);
    let mut harness = SchedulerHarness::new(64, 600);
    harness.plan(&build, &wanted);
    harness.report_jobs(true, 1000);

    harness.scheduler.invalidate(&wanted);
    harness.plan(&build, &wanted);

    assert_eq!(harness.job_keys(), wanted);
    assert_eq!(harness.frame.jobs[0].slot, 0);
    assert_eq!(harness.draw_slots(), [0]);
    harness.report_jobs(true, 2000);
    harness.plan(&build, &wanted);
    assert!(harness.frame.jobs.is_empty());
    assert_eq!(harness.scheduler.used_bytes(), 2000);
}

#[test]
fn a_lost_job_is_handed_out_again() {
    let build = SchedulerHarness::row(1);
    let wanted = SchedulerHarness::wanted(&build);
    let mut harness = SchedulerHarness::new(64, 600);
    harness.plan(&build, &wanted);
    let lost = harness.frame.jobs[0].serial;

    for _ in 1..GhostScheduler::PENDING_PLANS {
        harness.plan(&build, &wanted);
        assert!(harness.frame.jobs.is_empty());
    }
    harness.plan(&build, &wanted);

    assert_eq!(harness.job_keys(), wanted);
    assert_eq!(harness.frame.jobs[0].slot, 0);
    let current = harness.frame.jobs[0].serial;
    assert_ne!(current, lost);
    assert_eq!(harness.scheduler.live_slots(), 1);

    let mut late = GhostReport::new();
    late.built.push(SlotBuilt { slot: 0, serial: lost, generation: harness.frame.generation, bytes: 5000, ok: true });
    harness.scheduler.apply_report(&late);
    harness.plan(&build, &wanted);
    assert!(harness.frame.draws.is_empty());
    assert_eq!(harness.scheduler.used_bytes(), 0);

    let mut real = GhostReport::new();
    real.built.push(SlotBuilt {
        slot: 0,
        serial: current,
        generation: harness.frame.generation,
        bytes: 1000,
        ok: true,
    });
    harness.scheduler.apply_report(&real);
    harness.plan(&build, &wanted);
    assert_eq!(harness.draw_slots(), [0]);
    assert_eq!(harness.scheduler.used_bytes(), 1000);
}

#[test]
fn a_slot_freed_and_reused_in_one_frame_is_in_frees_and_jobs() {
    let build = SchedulerHarness::row(2);
    let wanted = SchedulerHarness::wanted(&build);
    let mut harness = SchedulerHarness::new(64, 600);
    harness.plan(&build, &wanted[..1]);
    harness.report_jobs(true, 1000);

    harness.plan(&build, &wanted[1..]);

    assert_eq!(harness.frame.frees, [0]);
    assert_eq!(harness.job_keys(), wanted[1..]);
    assert_eq!(harness.frame.jobs[0].slot, 0);
}

#[test]
fn empty_layer_jobs_still_spend_the_cell_budget() {
    let build = GhostFixture::blocks(&[([0, 0, 0], [0, 0, 0])]);
    let keys: Vec<SectionKey> = (0..8)
        .flat_map(|x| (0..16).map(move |y| SectionKey { x, y: 0, z: 0, layer: Some(y), part: SectionPart::Whole }))
        .collect();
    let mut harness = SchedulerHarness::new(64, 64);

    harness.plan(&build, &keys);

    assert_eq!(harness.frame.jobs.len(), 64);
}

#[test]
fn only_max_failed_keys_are_remembered() {
    let count = GhostScheduler::MAX_FAILED as i32 + 1;
    let build = SchedulerHarness::row(count);
    let wanted = SchedulerHarness::wanted_within(&build, 10_000);
    assert_eq!(wanted.len(), count as usize);
    let mut harness = SchedulerHarness::new(2000, 1_000_000);
    harness.plan(&build, &wanted);
    assert_eq!(harness.frame.jobs.len(), wanted.len());
    harness.report_jobs(false, 0);

    harness.plan(&build, &wanted);

    assert_eq!(harness.job_keys(), wanted[GhostScheduler::MAX_FAILED..]);
}

#[test]
fn an_unwanted_pending_slot_is_freed_and_its_late_report_is_dropped() {
    let build = SchedulerHarness::row(2);
    let wanted = SchedulerHarness::wanted(&build);
    let mut harness = SchedulerHarness::new(1, 600);
    harness.plan(&build, &wanted[..1]);
    let lost = harness.frame.jobs[0].serial;

    harness.plan(&build, &wanted[1..]);

    assert_eq!(harness.frame.frees, [0]);
    assert_eq!(harness.scheduler.live_slots(), 1);
    assert_eq!(harness.job_keys(), wanted[1..]);
    assert_eq!(harness.frame.jobs[0].slot, 0);
    let mut late = GhostReport::new();
    late.built.push(SlotBuilt { slot: 0, serial: lost, generation: harness.frame.generation, bytes: 5000, ok: true });
    harness.scheduler.apply_report(&late);
    harness.plan(&build, &wanted[1..]);
    assert!(harness.frame.draws.is_empty());
    assert_eq!(harness.scheduler.used_bytes(), 0);

    harness.plan(&build, &[]);
    assert_eq!(harness.scheduler.live_slots(), 0);
    harness.plan(&build, &wanted);
    assert_eq!(harness.frame.jobs.len(), 2);
}

#[test]
fn a_job_past_the_cell_budget_is_not_filled_again_next_plan() {
    let first = GhostFixture::blocks(&[([0, 0, 0], [4, 5, 0]), ([16, 0, 0], [24, 5, 0])]);
    let mut second = GhostFixture::blocks(&[([0, 0, 0], [4, 5, 0]), ([16, 0, 0], [16, 0, 0])]);
    second.revision = first.revision;
    let wanted = SchedulerHarness::wanted(&first);
    let mut harness = SchedulerHarness::new(64, 64);
    harness.plan(&first, &wanted);
    assert_eq!(harness.frame.jobs.len(), 1);
    harness.report_jobs(true, 1000);

    harness.plan(&second, &wanted);

    let mut expected = GhostJob::new(wanted[1]);
    GhostSections::fill_job(&first, wanted[1], &mut expected);
    assert_eq!(harness.job_keys(), wanted[1..]);
    assert_eq!(harness.frame.jobs[0].cells, expected.cells);
}

#[test]
fn retired_slots_stay_drawn_until_every_new_key_is_built() {
    let build = SchedulerHarness::row(2);
    let wanted = SchedulerHarness::wanted(&build);
    let mut harness = SchedulerHarness::new(64, 600);
    harness.plan(&build, &wanted);
    harness.report_jobs(true, 1000);
    harness.plan(&build, &wanted);
    let old = harness.draw_slots();
    assert_eq!(old.len(), 2);

    harness.scheduler.retire_all([0, 0, 0]);
    harness.plan(&build, &wanted);

    assert_eq!(harness.job_keys(), wanted);
    assert!(harness.frame.jobs.iter().all(|job| !old.contains(&job.slot)));
    assert_eq!(harness.draw_slots(), old);
    assert!(harness.frame.frees.is_empty());
    assert_eq!(harness.scheduler.retired_slots(), 2);

    harness.report_jobs(true, 1000);
    harness.plan(&build, &wanted);

    let mut frees = harness.frame.frees.clone();
    frees.sort();
    assert_eq!(frees, old);
    assert_eq!(harness.draw_slots().len(), 2);
    assert!(harness.draw_slots().iter().all(|slot| !old.contains(slot)));
    assert_eq!(harness.scheduler.retired_slots(), 0);
    assert_eq!(harness.scheduler.live_slots(), 2);
}

#[test]
fn a_full_slot_table_frees_a_retired_slot_for_each_new_key() {
    let mut harness = SchedulerHarness::new(1, 600);
    let capacity = harness.scheduler.slot_capacity();
    let build = SchedulerHarness::row(capacity as i32);
    let wanted = SchedulerHarness::wanted(&build);
    harness.plan(&build, &wanted);
    harness.report_jobs(true, 0);
    harness.plan(&build, &wanted);
    assert_eq!(harness.scheduler.live_slots(), capacity);

    harness.scheduler.retire_all([0, 0, 0]);
    harness.plan(&build, &wanted);

    let jobs = harness.frame.jobs.len() as u32;
    assert!(jobs > 0);
    assert_eq!(harness.frame.frees.len() as u32, jobs);
    assert_eq!(harness.scheduler.retired_slots(), capacity - jobs);
    assert_eq!(harness.scheduler.live_slots(), capacity);
    assert!(!harness.frame.cut);
}

#[test]
fn repeated_changes_before_the_reports_do_not_grow_the_retired_set_and_the_swap_finishes() {
    let build = SchedulerHarness::row(2);
    let wanted = SchedulerHarness::wanted(&build);
    let mut harness = SchedulerHarness::new(64, 600);
    harness.plan(&build, &wanted);
    harness.report_jobs(true, 1000);
    harness.plan(&build, &wanted);
    let old = harness.draw_slots();

    for _ in 0..3 {
        harness.scheduler.retire_all([0, 0, 0]);
        harness.plan(&build, &wanted);
        assert_eq!(harness.scheduler.retired_slots(), 2);
        assert_eq!(harness.draw_slots(), old);
        assert_eq!(harness.frame.jobs.len(), 2);
    }
    harness.scheduler.hold_unwanted();
    harness.plan(&build, &wanted);
    assert_eq!(harness.scheduler.retired_slots(), 2);
    assert_eq!(harness.draw_slots(), old);

    harness.scheduler.retire_all([0, 0, 0]);
    harness.plan(&build, &wanted);
    harness.report_jobs(true, 1000);
    harness.plan(&build, &wanted);

    assert_eq!(harness.scheduler.retired_slots(), 0);
    assert_eq!(harness.scheduler.live_slots(), 2);
    assert_eq!(harness.draw_slots().len(), 2);
    assert!(harness.draw_slots().iter().all(|slot| !old.contains(slot)));
}

#[test]
fn a_shift_moves_every_draw_and_a_retire_keeps_the_shifted_place() {
    let build = SchedulerHarness::row(2);
    let wanted = SchedulerHarness::wanted(&build);
    let mut harness = SchedulerHarness::new(64, 600);
    harness.plan(&build, &wanted);
    harness.report_jobs(true, 1000);
    harness.plan(&build, &wanted);
    let moved = |draws: &[GhostDraw], delta: [i32; 3]| -> Vec<GhostDraw> {
        draws
            .iter()
            .map(|draw| GhostDraw { origin: [0, 1, 2].map(|axis| draw.origin[axis] + delta[axis]), ..*draw })
            .collect()
    };
    let base = harness.frame.draws.clone();

    harness.plan_shifted(&build, &wanted, [2, 0, -1]);
    assert!(harness.frame.jobs.is_empty());
    assert!(harness.frame.frees.is_empty());
    assert_eq!(harness.frame.draws, moved(&base, [2, 0, -1]));

    harness.scheduler.retire_all([2, 0, -1]);
    harness.plan(&build, &wanted);
    assert_eq!(harness.frame.jobs.len(), 2);
    assert_eq!(harness.frame.draws, moved(&base, [2, 0, -1]));

    harness.plan_shifted(&build, &wanted, [1, 0, 0]);
    assert_eq!(harness.frame.draws, moved(&base, [3, 0, -1]));
}

#[test]
fn a_group_draws_scaled_at_world_min_beside_full_draws_at_scale_one() {
    let build = GhostFixture::blocks(&[([-20, 0, 0], [-20, 0, 0]), ([10, 0, 0], [10, 0, 0])]);
    let mut harness = SchedulerHarness::new(64, 600);
    harness.cut_groups(&build, &[2]);
    let full = SchedulerHarness::occupied(&build);
    let wanted: Vec<SectionKey> = harness.group_keys().iter().chain(&full).copied().collect();

    harness.settle(&build, &wanted, 1000);

    assert_eq!(harness.frame.draws.len(), wanted.len());
    let scaled: Vec<&GhostDraw> = harness.frame.draws.iter().filter(|draw| draw.scale != 1.0).collect();
    assert_eq!(scaled.len(), 1);
    assert_eq!(scaled[0].origin, build.world_min);
    assert_eq!(scaled[0].scale, GhostSections::miniature_scale(&build));
}

#[test]
fn undrawn_lead_keeps_slots_and_draws_none() {
    let build = SchedulerHarness::row(2);
    let mut harness = SchedulerHarness::new(64, 600);
    harness.cut_groups(&build, &[2]);
    let full = SchedulerHarness::occupied(&build);
    let wanted: Vec<SectionKey> = harness.group_keys().iter().chain(&full).copied().collect();
    harness.scheduler.set_undrawn_lead(1);
    harness.plan(&build, &wanted);
    let group_slot = harness.job_slots().into_iter().find(|(key, _)| *key == wanted[0]).map(|(_, slot)| slot).unwrap();
    harness.report_jobs(true, 1000);
    harness.settle(&build, &wanted, 1000);

    assert!(harness.frame.jobs.is_empty());
    assert_eq!(harness.frame.draws.len(), full.len());
    assert!(!harness.draw_slots().contains(&group_slot));

    harness.scheduler.set_undrawn_lead(0);
    harness.plan(&build, &wanted);

    assert!(harness.frame.jobs.is_empty());
    assert_eq!(harness.frame.draws.len(), wanted.len());
    assert!(harness.draw_slots().contains(&group_slot));
}

#[test]
fn a_group_slot_survives_a_budget_cut() {
    let build = SchedulerHarness::row(5);
    let mut harness = SchedulerHarness::new(1, 600);
    harness.cut_groups(&build, &[5]);
    let full = SchedulerHarness::occupied(&build);
    let wanted: Vec<SectionKey> = harness.group_keys().iter().chain(&full).copied().collect();
    assert_eq!(harness.scheduler.slot_capacity(), 4);

    harness.settle(&build, &wanted, 1000);

    assert!(harness.frame.cut);
    assert_eq!(harness.scheduler.miniature_group_slots(&harness.groups), (1, 5, 1000));

    let reversed: Vec<SectionKey> = harness.group_keys().iter().chain(full.iter().rev()).copied().collect();
    harness.plan(&build, &reversed);

    assert!(harness.frame.cut);
    assert!(harness.group_jobs().is_empty());
    assert_eq!(harness.scheduler.miniature_group_slots(&harness.groups), (1, 5, 1000));
}

#[test]
fn a_retire_does_not_redraw_an_undrawn_lead() {
    let build = SchedulerHarness::row(2);
    let mut harness = SchedulerHarness::new(64, 600);
    harness.cut_groups(&build, &[2]);
    let full = SchedulerHarness::occupied(&build);
    let wanted: Vec<SectionKey> = harness.group_keys().iter().chain(&full).copied().collect();
    harness.plan(&build, &wanted);
    let group_slot = harness.job_slots().into_iter().find(|(key, _)| *key == wanted[0]).map(|(_, slot)| slot).unwrap();
    harness.report_jobs(true, 1000);
    harness.settle(&build, &wanted, 1000);
    assert!(harness.draw_slots().contains(&group_slot));

    harness.scheduler.set_undrawn_lead(1);
    harness.plan(&build, &wanted);
    harness.scheduler.retire_all([0, 0, 0]);
    harness.plan(&build, &wanted);

    assert!(!harness.draw_slots().contains(&group_slot));
    harness.settle(&build, &wanted, 1000);
    assert!(harness.draw_slots().iter().all(|slot| *slot != group_slot));
    assert_eq!(harness.draw_slots().len(), full.len());
}

#[test]
fn a_group_is_appends_in_order_then_a_finish() {
    let build = SchedulerHarness::row(3);
    let mut harness = SchedulerHarness::new(64, 600);
    harness.cut_groups(&build, &[3]);
    let wanted = harness.group_keys();
    let members = harness.groups.members(0).to_vec();
    let size = [0, 1, 2].map(|axis| (build.world_max[axis] - build.world_min[axis] + 1) as u32);
    let mut seen = Vec::new();

    for _ in 0..4 {
        harness.plan(&build, &wanted);
        assert_eq!(harness.frame.jobs.len(), 1);
        let job = &harness.frame.jobs[0];
        seen.push((job.mode, job.group_index, job.group_serial, job.slot));
        if job.mode == GhostJobMode::Append {
            let member = members[job.group_index as usize];
            let base = [member.x * 16, member.y * 16 + member.part.y_offset(), member.z * 16];
            assert_eq!(job.origin, build.world_min);
            assert_eq!(job.group_offset, [0, 1, 2].map(|axis| base[axis] - build.world_min[axis]));
            assert_eq!(job.slot_size, size);
        }
        harness.report_jobs(true, 100);
    }

    let modes: Vec<_> = seen.iter().map(|entry| entry.0).collect();
    assert_eq!(modes, [GhostJobMode::Append, GhostJobMode::Append, GhostJobMode::Append, GhostJobMode::Finish]);
    assert_eq!(seen.iter().map(|entry| entry.1).collect::<Vec<_>>(), [0, 1, 2, 3]);
    assert!(seen.iter().all(|entry| entry.2 == seen[0].2 && entry.3 == seen[0].3));
}

#[test]
fn only_one_group_job_is_in_flight() {
    let boxes: Vec<([i32; 3], [i32; 3])> = (0..10).map(|n| ([n * 16, 0, 0], [n * 16 + 4, 3, 0])).collect();
    let build = GhostFixture::blocks(&boxes);
    let mut harness = SchedulerHarness::new(64, 64);
    let occupied = SchedulerHarness::occupied(&build);
    harness.groups.cut(&occupied[..4], &[10_000, 1, 10_000, 1]);
    assert_eq!(harness.groups.group_count(), 2);
    let mut wanted = harness.group_keys();
    wanted.extend(occupied.iter().copied());

    harness.plan(&build, &wanted);
    assert_eq!(harness.group_jobs().len(), 1);
    assert!(harness.frame.jobs.len() > 1);
    harness.report_matching(|job| job.key.part != SectionPart::MiniatureGroup, true, 100);
    for _ in 0..3 {
        harness.plan(&build, &wanted);
        assert!(harness.group_jobs().is_empty());
        assert!(!harness.frame.jobs.is_empty());
        harness.report_jobs(true, 100);
    }

    let mut group_jobs = 0;
    for _ in 0..12 {
        harness.plan(&build, &wanted);
        assert!(harness.group_jobs().len() <= 1);
        group_jobs += harness.group_jobs().len();
        harness.report_jobs(true, 100);
    }
    assert!(group_jobs > 0);
}

#[test]
fn the_next_group_waits_for_the_finish() {
    let build = SchedulerHarness::row(4);
    let mut harness = SchedulerHarness::new(64, 600);
    harness.cut_groups(&build, &[2, 2]);
    let wanted = harness.group_keys();

    loop {
        harness.plan(&build, &wanted);
        assert_eq!(harness.frame.jobs[0].key, wanted[0]);
        if harness.frame.jobs[0].mode == GhostJobMode::Finish {
            break;
        }
        harness.report_jobs(true, 100);
    }
    harness.plan(&build, &wanted);
    assert!(harness.frame.jobs.is_empty());
    harness.plan(&build, &wanted);
    assert!(harness.frame.jobs.is_empty());
}

#[test]
fn the_next_group_starts_after_the_finish_is_reported() {
    let build = SchedulerHarness::row(4);
    let mut harness = SchedulerHarness::new(64, 600);
    harness.cut_groups(&build, &[2, 2]);
    let wanted = harness.group_keys();

    loop {
        harness.plan(&build, &wanted);
        let finish = harness.frame.jobs[0].mode == GhostJobMode::Finish;
        harness.report_jobs(true, 100);
        if finish {
            break;
        }
    }
    harness.plan(&build, &wanted);

    assert_eq!(harness.frame.jobs.len(), 1);
    assert_eq!(harness.frame.jobs[0].key, wanted[1]);
    assert_eq!(harness.frame.jobs[0].group_index, 0);
}

#[test]
fn a_group_is_drawn_only_after_its_finish() {
    let build = SchedulerHarness::row(2);
    let mut harness = SchedulerHarness::new(64, 600);
    harness.cut_groups(&build, &[2]);
    let wanted = harness.group_keys();

    for _ in 0..2 {
        harness.plan(&build, &wanted);
        assert!(harness.frame.draws.is_empty());
        harness.report_jobs(true, 100);
    }
    harness.plan(&build, &wanted);
    assert_eq!(harness.frame.jobs[0].mode, GhostJobMode::Finish);
    assert!(harness.frame.draws.is_empty());
    harness.report_jobs(true, 100);
    harness.plan(&build, &wanted);

    let size_x = build.world_max[0] - build.world_min[0] + 1;
    let size_z = build.world_max[2] - build.world_min[2] + 1;
    assert_eq!(harness.frame.draws.len(), 1);
    assert_eq!(harness.frame.draws[0].origin, build.world_min);
    assert_eq!(harness.frame.draws[0].offset, [-size_x, 0, -size_z]);
    assert!((harness.frame.draws[0].scale - GhostSections::miniature_scale(&build)).abs() < 1e-6);
}

#[test]
fn a_lost_group_report_hands_out_the_same_job() {
    let build = SchedulerHarness::row(2);
    let mut harness = SchedulerHarness::new(64, 600);
    harness.cut_groups(&build, &[2]);
    let wanted = harness.group_keys();
    harness.plan(&build, &wanted);
    let first = harness.frame.jobs[0].clone();

    for _ in 0..GhostScheduler::PENDING_PLANS - 1 {
        harness.plan(&build, &wanted);
        assert!(harness.frame.jobs.is_empty());
    }
    harness.plan(&build, &wanted);

    assert_eq!(harness.frame.jobs.len(), 1);
    let again = &harness.frame.jobs[0];
    assert_eq!(
        (again.group_serial, again.group_index, again.slot),
        (first.group_serial, first.group_index, first.slot)
    );
    assert_ne!(again.serial, first.serial);
}

#[test]
fn a_second_lost_report_at_one_index_restarts_the_group() {
    let build = SchedulerHarness::row(3);
    let mut harness = SchedulerHarness::new(64, 600);
    harness.cut_groups(&build, &[3]);
    let wanted = harness.group_keys();
    harness.plan(&build, &wanted);
    let first_serial = harness.frame.jobs[0].group_serial;
    harness.report_jobs(true, 100);
    harness.plan(&build, &wanted);
    assert_eq!(harness.frame.jobs[0].group_index, 1);

    for _ in 0..GhostScheduler::PENDING_PLANS - 1 {
        harness.plan(&build, &wanted);
        assert!(harness.frame.jobs.is_empty());
    }
    harness.plan(&build, &wanted);
    assert_eq!(harness.frame.jobs[0].group_index, 1);
    assert_eq!(harness.frame.jobs[0].group_serial, first_serial);

    for _ in 0..GhostScheduler::PENDING_PLANS - 1 {
        harness.plan(&build, &wanted);
        assert!(harness.frame.jobs.is_empty());
    }
    harness.plan(&build, &wanted);
    assert_eq!(harness.frame.jobs.len(), 1);
    assert_eq!(harness.frame.jobs[0].mode, GhostJobMode::Append);
    assert_eq!(harness.frame.jobs[0].group_index, 0);
    assert_ne!(harness.frame.jobs[0].group_serial, first_serial);

    harness.report_jobs(true, 100);
    harness.settle(&build, &wanted, 100);
    assert_eq!(harness.frame.draws.len(), 1);
}

#[test]
fn a_later_group_builds_after_a_restart() {
    let build = SchedulerHarness::row(4);
    let mut harness = SchedulerHarness::new(64, 600);
    harness.cut_groups(&build, &[2, 2]);
    let wanted = harness.group_keys();
    harness.plan(&build, &wanted);
    harness.report_jobs(true, 100);
    harness.plan(&build, &wanted);
    assert_eq!(harness.frame.jobs[0].group_index, 1);

    for _ in 0..2 * GhostScheduler::PENDING_PLANS {
        harness.plan(&build, &wanted);
    }
    assert_eq!(harness.frame.jobs[0].group_index, 0);
    assert_eq!(harness.frame.jobs[0].key, wanted[0]);

    harness.report_jobs(true, 100);
    harness.settle(&build, &wanted, 100);
    assert_eq!(harness.frame.draws.len(), 2);
}

#[test]
fn an_invalidate_mid_group_restarts_it() {
    let build = SchedulerHarness::row(2);
    let mut harness = SchedulerHarness::new(64, 600);
    harness.cut_groups(&build, &[2]);
    let wanted = harness.group_keys();
    harness.plan(&build, &wanted);
    let first_serial = harness.frame.jobs[0].group_serial;
    harness.report_jobs(true, 100);

    harness.scheduler.invalidate_all();
    harness.plan(&build, &wanted);

    assert_eq!(harness.frame.jobs[0].group_index, 0);
    assert_ne!(harness.frame.jobs[0].group_serial, first_serial);

    for _ in 0..2 {
        harness.report_jobs(true, 100);
        harness.plan(&build, &wanted);
    }
    assert_eq!(harness.frame.jobs[0].mode, GhostJobMode::Finish);
    harness.report_jobs(true, 100);
    harness.plan(&build, &wanted);
    assert_eq!(harness.frame.draws.len(), 1);
    let restarted = harness.frame.jobs.first().map(|job| job.group_index);
    assert_eq!(restarted, None);

    harness.scheduler.invalidate_all();
    harness.plan(&build, &wanted);
    assert_eq!(harness.frame.jobs[0].group_index, 0);
    assert_eq!(harness.frame.draws.len(), 1);
    harness.report_jobs(true, 100);
    harness.plan(&build, &wanted);
    assert_eq!(harness.frame.jobs[0].group_index, 1);
    assert_eq!(harness.frame.draws.len(), 1);
}

#[test]
fn retire_all_mid_group_starts_a_new_build() {
    let build = SchedulerHarness::row(2);
    let mut harness = SchedulerHarness::new(64, 600);
    harness.cut_groups(&build, &[2]);
    let wanted = harness.group_keys();
    harness.plan(&build, &wanted);
    let first_serial = harness.frame.jobs[0].group_serial;
    harness.report_jobs(true, 100);

    harness.scheduler.retire_all([0; 3]);
    harness.plan(&build, &wanted);

    assert_eq!(harness.frame.jobs.len(), 1);
    assert_eq!(harness.frame.jobs[0].group_index, 0);
    assert_ne!(harness.frame.jobs[0].group_serial, first_serial);
    for _ in 0..2 {
        harness.report_jobs(true, 100);
        harness.plan(&build, &wanted);
    }
    harness.report_jobs(true, 100);
    harness.plan(&build, &wanted);
    assert_eq!(harness.frame.draws.len(), 1);
}

#[test]
fn a_failed_append_fails_its_group_only() {
    let build = SchedulerHarness::row(4);
    let mut harness = SchedulerHarness::new(64, 600);
    harness.cut_groups(&build, &[2, 2]);
    let wanted = harness.group_keys();
    harness.plan(&build, &wanted);
    assert_eq!(harness.frame.jobs[0].key, wanted[0]);
    harness.report_jobs(false, 0);

    let mut keys = Vec::new();
    for _ in 0..6 {
        harness.plan(&build, &wanted);
        keys.extend(harness.job_keys());
        harness.report_jobs(true, 100);
    }
    harness.plan(&build, &wanted);

    assert!(keys.iter().all(|key| *key == wanted[1]));
    assert_eq!(keys.len(), 3);
    assert_eq!(harness.frame.draws.len(), 1);
}

#[test]
fn miniature_slots_counts_finished_groups() {
    let build = SchedulerHarness::row(5);
    let mut harness = SchedulerHarness::new(64, 600);
    harness.cut_groups(&build, &[2, 3]);
    let wanted = harness.group_keys();

    for _ in 0..8 {
        harness.plan(&build, &wanted);
        let bytes = if harness.frame.jobs.first().is_some_and(|job| job.key == wanted[0]) { 100 } else { 200 };
        harness.report_jobs(true, bytes);
    }

    assert_eq!(harness.scheduler.miniature_group_slots(&harness.groups), (2, 5, 300));
}

#[test]
fn a_generation_change_mid_group_builds_the_next_generations_groups_from_index_zero() {
    let build = SchedulerHarness::row(3);
    let mut harness = SchedulerHarness::new(64, 600);
    harness.cut_groups(&build, &[3]);
    let wanted = harness.group_keys();
    harness.plan(&build, &wanted);
    let first_serial = harness.frame.jobs[0].group_serial;
    harness.report_jobs(true, 100);
    harness.plan(&build, &wanted);
    assert_eq!(harness.frame.jobs[0].group_index, 1);
    let mut second = build.clone();
    second.revision = 2;

    harness.plan(&second, &wanted);

    assert!(harness.frame.clear_all);
    assert_eq!(harness.frame.jobs.len(), 1);
    assert_eq!(harness.frame.jobs[0].mode, GhostJobMode::Append);
    assert_eq!(harness.frame.jobs[0].group_index, 0);
    assert_ne!(harness.frame.jobs[0].group_serial, first_serial);
    harness.report_jobs(true, 100);
    harness.settle(&second, &wanted, 100);
    assert_eq!(harness.frame.draws.len(), 1);
}

#[test]
fn a_failed_finish_fails_its_group_only() {
    let build = SchedulerHarness::row(4);
    let mut harness = SchedulerHarness::new(64, 600);
    harness.cut_groups(&build, &[2, 2]);
    let wanted = harness.group_keys();
    loop {
        harness.plan(&build, &wanted);
        let finish = harness.frame.jobs[0].mode == GhostJobMode::Finish;
        assert_eq!(harness.frame.jobs[0].key, wanted[0]);
        harness.report_jobs(!finish, 100);
        if finish {
            break;
        }
    }

    let mut keys = Vec::new();
    for _ in 0..5 {
        harness.plan(&build, &wanted);
        keys.extend(harness.job_keys());
        harness.report_jobs(true, 100);
    }
    harness.plan(&build, &wanted);

    assert_eq!(keys.len(), 3);
    assert!(keys.iter().all(|key| *key == wanted[1]));
    assert_eq!(harness.frame.draws.len(), 1);
    assert_eq!(harness.scheduler.miniature_group_slots(&harness.groups), (1, 2, 100));
}

#[test]
fn retire_all_while_a_built_group_rebuilds_keeps_the_old_mesh_drawn_until_the_finish() {
    let build = SchedulerHarness::row(2);
    let mut harness = SchedulerHarness::new(64, 600);
    harness.cut_groups(&build, &[2]);
    let wanted = harness.group_keys();
    harness.settle(&build, &wanted, 100);
    assert_eq!(harness.frame.draws.len(), 1);
    let old = harness.frame.draws[0].slot;

    harness.scheduler.invalidate_all();
    harness.plan(&build, &wanted);
    harness.report_jobs(true, 100);
    harness.scheduler.retire_all([0; 3]);
    for _ in 0..3 {
        harness.plan(&build, &wanted);
        assert_eq!(harness.frame.jobs.len(), 1);
        assert_ne!(harness.frame.jobs[0].slot, old);
        assert_eq!(harness.draw_slots(), [old]);
        harness.report_jobs(true, 100);
    }
    harness.plan(&build, &wanted);

    assert!(harness.frame.jobs.is_empty());
    assert_eq!(harness.frame.draws.len(), 1);
    assert_ne!(harness.frame.draws[0].slot, old);
}

#[test]
fn fewer_groups_while_one_builds_release_its_slot_and_the_rest_still_finish() {
    let build = SchedulerHarness::row(4);
    let mut harness = SchedulerHarness::new(64, 600);
    harness.cut_groups(&build, &[2, 2]);
    let wanted = harness.group_keys();
    loop {
        harness.plan(&build, &wanted);
        let finish = harness.frame.jobs[0].mode == GhostJobMode::Finish;
        harness.report_jobs(true, 100);
        if finish {
            break;
        }
    }
    harness.plan(&build, &wanted);
    assert_eq!(harness.frame.jobs[0].key, wanted[1]);
    let released = harness.frame.jobs[0].slot;
    harness.report_jobs(true, 100);

    harness.cut_groups(&build, &[2]);
    let fewer = harness.group_keys();
    harness.plan(&build, &fewer);

    assert_eq!(fewer.len(), 1);
    assert!(harness.frame.frees.contains(&released));
    assert!(harness.frame.jobs.is_empty());
    assert_eq!(harness.frame.draws.len(), 1);

    harness.cut_groups(&build, &[2, 2]);
    harness.settle(&build, &wanted, 100);

    assert_eq!(harness.frame.draws.len(), 2);
    assert_eq!(harness.scheduler.miniature_group_slots(&harness.groups), (2, 4, 200));
}
