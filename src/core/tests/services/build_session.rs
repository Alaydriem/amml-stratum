use stratum_core::services::BuildOrderWorker;
use stratum_core::structs::build::{
    BuildAnswer, BuildJob, BuildKindsResult, BuildNotice, BuildPhase, BuildSettings, BuildStopReason,
};
use stratum_core::structs::marker::Rotation;

use crate::support::{BuildFixture, BuildRun, GhostFixture, Pieces};

const RESTORE: &str = "/gamerule sendcommandfeedback true";
const SILENCE: &str = "/gamerule sendcommandfeedback false";

fn row_of(width: usize) -> stratum_core::structs::ghost::GhostBuild {
    let row = "#".repeat(width);
    BuildFixture::build(Pieces::layer_at(&[row.as_str()], [0, 64, 0]), Rotation::R0)
}

fn rate(blocks_per_second: u8) -> BuildSettings {
    BuildSettings { blocks_per_second, ..BuildSettings::default() }
}

fn setblocks(commands: &[String]) -> Vec<&String> {
    commands.iter().filter(|command| BuildRun::is_setblock(command)).collect()
}

#[test]
fn rate_4_sends_4_setblocks_per_second_after_the_clear() {
    let mut run = BuildRun::ready(&row_of(8), rate(4));
    run.start_and_clear();

    let mut sent = Vec::new();
    for _ in 0..4 {
        sent.extend(run.tick(0.25));
    }

    assert_eq!(setblocks(&sent).len(), 4);
    assert_eq!(
        sent.iter().map(|command| BuildFixture::position(command)).collect::<Vec<_>>(),
        vec![[0, 64, 0], [1, 64, 0], [2, 64, 0], [3, 64, 0]]
    );
    assert_eq!(run.row().placed, 4);
}

#[test]
fn rate_60_sends_60_setblocks_per_second_at_20_ticks_a_second() {
    let mut run = BuildRun::ready(&row_of(120), rate(60));
    run.start_and_clear();

    let mut sent = Vec::new();
    for _ in 0..20 {
        sent.extend(run.tick(0.05));
    }

    assert_eq!(setblocks(&sent).len(), 60);
    assert_eq!(run.row().placed, 60);
}

#[test]
fn the_clear_sends_one_fill_per_tick_before_any_setblock() {
    let build = GhostFixture::blocks(&[([0, 64, 0], [31, 95, 32])]);
    let mut run = BuildRun::ready(&build, rate(20));
    run.start();
    assert_eq!(run.drain(), vec![SILENCE.to_string()]);

    let first = run.tick(1.0);
    assert_eq!(first.len(), 1);
    assert!(first[0].starts_with("/fill "));
    assert_eq!(run.phase(), BuildPhase::Clearing);

    let second = run.tick(1.0);
    assert_eq!(second.len(), 1);
    assert!(second[0].starts_with("/fill "));
    assert_eq!(run.phase(), BuildPhase::Building);

    let third = run.tick(0.25);
    assert!(setblocks(&third).len() == third.len() && !third.iter().any(|command| command.starts_with("/fill ")));
}

#[test]
fn many_frames_inside_one_game_tick_send_one_fill() {
    // Three boxes: a 32 x 32 x 96 volume needs three fills.
    let build = GhostFixture::blocks(&[([0, 64, 0], [31, 159, 32])]);
    let mut run = BuildRun::ready(&build, rate(20));
    run.start();
    assert_eq!(run.drain(), vec![SILENCE.to_string()]);

    // 144 frames a second: seven frames fit into one game tick of 0.05 s.
    let mut fills = Vec::new();
    for _ in 0..4 {
        fills.extend(run.tick(0.007));
    }
    assert_eq!(fills.len(), 1, "four frames in the same game tick: {fills:?}");

    // A full game tick later the next fill goes out.
    fills.extend(run.tick(0.05));
    assert_eq!(fills.len(), 2);
    assert!(fills.iter().all(|command| command.starts_with("/fill ")));
}

#[test]
fn eta_is_remaining_over_rate_and_stops_while_paused() {
    let mut run = BuildRun::ready(&row_of(40), rate(4));
    assert_eq!(run.row().total, 40);
    assert_eq!(run.row().eta_seconds, 10);

    run.start_and_clear();
    run.tick(1.0);
    assert_eq!(run.row().placed, 4);
    assert_eq!(run.row().eta_seconds, 9);

    run.session.pause(run.time);
    assert_eq!(run.phase(), BuildPhase::Paused);
    let during = run.tick(5.0);
    assert!(during.is_empty());
    assert_eq!(run.row().eta_seconds, 9);
    assert_eq!(run.row().placed, 4);

    run.session.resume(run.time);
    let after = run.tick(1.0);
    assert_eq!(after.len(), 4);
    assert_eq!(run.row().eta_seconds, 8);
}

#[test]
fn layers_go_bottom_up_and_fluids_come_last() {
    let piece = Pieces::layers_at(&[&["#w#"], &["#s#"], &["#t#"]], [0, 64, 0]);
    let build = BuildFixture::build(piece, Rotation::R0);
    let mut run = BuildRun::ready(&build, rate(1));
    assert_eq!(run.row().total, 9);
    run.start_and_clear();

    // One block a second. Each empty layer of the hanging pass and the first layer of the fluid pass wait one tick.
    let mut placed = Vec::new();
    for _ in 0..32 {
        placed.extend(run.tick(1.0));
        if placed.len() == 7 {
            break;
        }
    }
    assert_eq!(
        placed.iter().map(|command| BuildFixture::position(command)).collect::<Vec<_>>(),
        vec![[0, 64, 0], [2, 64, 0], [0, 65, 0], [2, 65, 0], [0, 66, 0], [2, 66, 0], [1, 66, 0]]
    );

    let mut water = Vec::new();
    for _ in 0..8 {
        water = run.tick(1.0);
        if !water.is_empty() {
            break;
        }
    }
    assert_eq!(run.phase(), BuildPhase::Fluids);
    assert_eq!(water.iter().map(|command| BuildFixture::position(command)).collect::<Vec<_>>(), vec![[1, 64, 0]]);
    assert!(water[0].contains("minecraft:water"));

    let sand = run.tick(1.0);
    let sand = setblocks(&sand);
    assert_eq!(sand.iter().map(|command| BuildFixture::position(command)).collect::<Vec<_>>(), vec![[1, 65, 0]]);
    assert!(sand[0].contains("minecraft:sand"));
    assert_eq!(run.phase(), BuildPhase::Done);
}

#[test]
fn a_layer_that_is_not_ready_sends_nothing_and_does_not_count_against_the_pace() {
    let piece = Pieces::layers_at(&[&["####"], &["####"]], [0, 64, 0]);
    let build = BuildFixture::build(piece, Rotation::R0);
    let mut run = BuildRun::ready(&build, rate(4));
    run.start_and_clear();
    run.auto = false;

    // The first layer is worked out already (asked for at the start); the second is asked for and not answered.
    run.pump();
    assert_eq!(setblocks(&run.tick(1.0)).len(), 4);
    let request = run.session.take_order_request().expect("the second layer is asked for");
    assert_eq!(request.layer, 1);
    assert!(run.session.take_order_request().is_none());

    // Five seconds pass with no layer: nothing is sent.
    assert!(run.tick(5.0).is_empty());
    assert_eq!(run.row().placed, 4);

    // The layer arrives. The wait is not made up with a burst: one second of pace gives 4 blocks, not 24.
    run.session.apply_order(BuildOrderWorker::work(request));
    assert_eq!(setblocks(&run.tick(0.25)).len(), 1);
    assert_eq!(run.row().placed, 5);
}

#[test]
fn an_answer_to_an_old_request_is_dropped() {
    let piece = Pieces::layers_at(&[&["##"], &["##"]], [0, 64, 0]);
    let build = BuildFixture::build(piece, Rotation::R0);
    let mut run = BuildRun::ready(&build, rate(1));
    run.auto = false;
    run.start();
    let old = run.session.take_order_request().unwrap();
    run.session.release_order();
    let current = run.session.take_order_request().unwrap();
    assert_ne!(old.serial, current.serial);

    run.session.apply_order(BuildOrderWorker::work(old));
    assert!(run.session.take_order_request().is_none());
    run.session.apply_order(BuildOrderWorker::work(current));
    // The layer is in: the session asks for no layer until it has loaded this one.
    assert!(run.session.take_order_request().is_none());
}

#[test]
fn the_order_worker_refuses_requests_past_its_queue_and_answers_the_rest() {
    let piece = std::sync::Arc::new(Pieces::layer_at(&["##"], [0, 64, 0]));
    let request = |serial| stratum_core::structs::build::OrderRequest {
        serial,
        pass: stratum_core::structs::build::BuildPass::Main,
        layer: 0,
        piece: piece.clone(),
        skip: std::sync::Arc::new(Vec::new()),
    };
    let parked = BuildOrderWorker::without_thread();
    for serial in 0..BuildOrderWorker::QUEUE as u64 {
        assert!(parked.submit(BuildJob::Order(request(serial))).is_ok());
    }
    assert!(matches!(parked.submit(BuildJob::Order(request(99))), Err(BuildJob::Order(back)) if back.serial == 99));

    let worker = BuildOrderWorker::new();
    assert!(worker.submit(BuildJob::Order(request(7))).is_ok());
    let mut answers = Vec::new();
    for _ in 0..2000 {
        worker.drain(&mut answers);
        if !answers.is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert_eq!(answers.len(), 1);
    assert!(matches!(&answers[0], BuildAnswer::Order(result) if result.serial == 7 && result.steps.len() == 2));
}

#[test]
fn a_structure_with_no_placeable_block_cannot_start() {
    let build = BuildFixture::build(Pieces::layer_at(&["ww"], [0, 64, 0]), Rotation::R0);
    let probe = BuildFixture::unknown(&build, &["water"]);
    let mut run = BuildRun::probed(&build, rate(4), &probe);
    assert_eq!(run.phase(), BuildPhase::Ready);
    assert_eq!(run.row().total, 0);

    run.start();

    assert_eq!(run.phase(), BuildPhase::Ready);
    assert!(run.drain().is_empty());
    assert!(run.session.take_kinds().is_none());
}

#[test]
fn unknown_entries_are_skipped_and_listed() {
    let build = BuildFixture::build(Pieces::layer_at(&["#d#d"], [0, 64, 0]), Rotation::R0);
    let probe = BuildFixture::unknown(&build, &["dirt"]);
    let mut run = BuildRun::probed(&build, rate(20), &probe);

    assert_eq!(run.row().total, 2);
    assert_eq!(run.row().unplaceable, vec!["minecraft:dirt".to_string()]);

    run.start_and_clear();
    let sent = run.tick(1.0);
    let placed = setblocks(&sent);
    assert_eq!(placed.len(), 2);
    assert!(placed.iter().all(|command| command.contains("minecraft:stone")));
    assert_eq!(run.phase(), BuildPhase::Done);
}

#[test]
fn the_upper_half_of_a_door_is_left_to_the_lower_half() {
    // the game places both halves from the lower half; a second /setblock of the upper half fails in the chat
    let build = BuildFixture::build(Pieces::layers_at(&[&["L#"], &["U."]], [0, 64, 0]), Rotation::R0);
    let probe = BuildFixture::exact(&build);
    let mut run = BuildRun::probed(&build, rate(20), &probe);

    assert_eq!(run.row().total, 2);
    run.start_and_clear();
    let sent = run.tick(1.0);
    let placed = setblocks(&sent);
    assert_eq!(placed.len(), 2);
    assert!(placed.iter().all(|command| !command.contains("\"upper_block_bit\"=true")), "{placed:?}");
    assert_eq!(run.phase(), BuildPhase::Done);
}

#[test]
fn name_only_blocks_are_placed_without_states() {
    use stratum_core::structs::build::{BuildProbeResult, PaletteOutcome};
    let build = BuildFixture::build(Pieces::layer_at(&["#"], [0, 64, 0]), Rotation::R0);
    let probe = BuildProbeResult {
        outcomes: vec![PaletteOutcome::NameOnly; build.palette.entries.len()],
        turn_mismatches: Vec::new(),
    };
    let mut run = BuildRun::probed(&build, rate(20), &probe);
    run.start_and_clear();
    let sent = run.tick(1.0);
    assert_eq!(setblocks(&sent), vec![&"/setblock 0 64 0 minecraft:stone".to_string()]);
}

#[test]
fn every_end_but_leaving_the_world_puts_the_game_rule_back() {
    let mut done = BuildRun::ready(&row_of(4), rate(20));
    done.start_and_clear();
    let mut commands = done.tick(1.0);
    commands.extend(done.drain());
    assert_eq!(done.phase(), BuildPhase::Done);
    assert_eq!(commands.last().map(String::as_str), Some(RESTORE));
    assert_eq!(setblocks(&commands).len(), 4);

    for reason in [BuildStopReason::Operator, BuildStopReason::ConditionLost, BuildStopReason::SendFailed] {
        let mut run = BuildRun::ready(&row_of(40), rate(4));
        run.start_and_clear();
        run.time += 1.0;
        run.session.tick(run.time, &mut run.outbox);
        assert!(!run.outbox.is_empty());
        run.stop(reason);
        assert_eq!(run.phase(), BuildPhase::Stopped(reason));
        // The blocks that waited for the mod are dropped; only the restore is left.
        assert_eq!(run.drain(), vec![RESTORE.to_string()]);
    }

    let mut left = BuildRun::ready(&row_of(40), rate(4));
    left.start_and_clear();
    left.time += 1.0;
    left.session.tick(left.time, &mut left.outbox);
    assert!(!left.outbox.is_empty());
    left.stop(BuildStopReason::LeftWorld);
    assert_eq!(left.phase(), BuildPhase::Stopped(BuildStopReason::LeftWorld));
    assert!(left.drain().is_empty());
}

#[test]
fn a_stop_before_the_start_sends_nothing() {
    let mut run = BuildRun::ready(&row_of(8), rate(4));
    run.stop(BuildStopReason::Operator);
    assert_eq!(run.phase(), BuildPhase::Ready);
    assert!(run.drain().is_empty());
}

#[test]
fn kept_water_removes_water_kinds_from_the_clear() {
    let settings = BuildSettings { blocks_per_second: 4, clear_water: false, clear_lava: true };
    let build = row_of(8);
    let mut run = BuildRun::ready(&build, settings);

    run.start();
    // Nothing is sent until the kinds are known.
    assert!(run.drain().is_empty());
    assert_eq!(run.phase(), BuildPhase::Probing);
    let request = run.session.take_kinds().unwrap();
    assert_eq!((request.min, request.max), ([0, 64, 0], [7, 64, 0]));

    let kinds = ["minecraft:stone", "minecraft:water", "minecraft:flowing_water", "minecraft:lava", "minecraft:air"];
    let kinds = BuildKindsResult::Kinds(kinds.iter().map(|kind| kind.to_string()).collect());
    run.session.apply_kinds(kinds, run.time, &mut run.outbox);
    assert_eq!(run.phase(), BuildPhase::Clearing);

    let mut sent = run.drain();
    sent.extend(run.tick(0.25));
    sent.extend(run.tick(0.25));
    assert_eq!(
        sent,
        vec![
            SILENCE.to_string(),
            "/fill 0 64 0 7 64 0 air replace minecraft:stone".to_string(),
            "/fill 0 64 0 7 64 0 air replace minecraft:lava".to_string(),
        ]
    );
    assert_eq!(run.phase(), BuildPhase::Building);
}

#[test]
fn kept_lava_removes_lava_kinds_from_the_clear() {
    let settings = BuildSettings { blocks_per_second: 4, clear_water: true, clear_lava: false };
    let mut run = BuildRun::ready(&row_of(8), settings);
    run.start();
    run.session.take_kinds().unwrap();
    let kinds = ["minecraft:flowing_lava", "minecraft:water", "minecraft:lava"];
    run.session.apply_kinds(
        BuildKindsResult::Kinds(kinds.iter().map(|kind| kind.to_string()).collect()),
        run.time,
        &mut run.outbox,
    );
    let mut sent = run.drain();
    sent.extend(run.tick(0.25));
    assert_eq!(sent, vec![SILENCE.to_string(), "/fill 0 64 0 7 64 0 air replace minecraft:water".to_string()]);
}

#[test]
fn not_loaded_returns_to_ready_with_the_message() {
    let settings = BuildSettings { blocks_per_second: 4, clear_water: false, clear_lava: false };
    let mut run = BuildRun::ready(&row_of(8), settings);
    run.start();
    run.session.take_kinds().unwrap();

    run.session.apply_kinds(BuildKindsResult::NotLoaded, run.time, &mut run.outbox);

    assert_eq!(run.phase(), BuildPhase::Ready);
    assert_eq!(run.row().notice, Some(BuildNotice::StandNear));
    assert!(run.drain().is_empty());

    run.start();
    run.session.take_kinds().unwrap();
    run.session.apply_kinds(BuildKindsResult::TooMany, run.time, &mut run.outbox);
    assert_eq!(run.phase(), BuildPhase::Ready);
    assert_eq!(run.row().notice, Some(BuildNotice::TooManyKinds));
}

#[test]
fn more_kinds_than_the_capacity_count_as_too_many() {
    let settings = BuildSettings { blocks_per_second: 4, clear_water: false, clear_lava: true };
    let mut run = BuildRun::ready(&row_of(8), settings);
    run.start();
    run.session.take_kinds().unwrap();
    let kinds = (0..4097).map(|n| format!("test:block_{n}")).collect();

    run.session.apply_kinds(BuildKindsResult::Kinds(kinds), run.time, &mut run.outbox);

    assert_eq!(run.phase(), BuildPhase::Ready);
    assert_eq!(run.row().notice, Some(BuildNotice::TooManyKinds));
    assert!(run.drain().is_empty());
}

#[test]
fn the_outbox_never_holds_more_than_its_capacity() {
    let mut run = BuildRun::ready(&row_of(300), rate(20));
    run.start_and_clear();
    run.drain();

    // A ten second frame gap makes 200 blocks due; only what fits is queued.
    run.time += 10.0;
    run.session.tick(run.time, &mut run.outbox);
    assert!(run.outbox.len() <= run.outbox.capacity());
    assert_eq!(run.outbox.capacity(), 64);

    // Nothing is taken: the outbox stays full and no more is queued.
    let waiting = run.outbox.len();
    run.time += 1.0;
    run.session.tick(run.time, &mut run.outbox);
    assert_eq!(run.outbox.len(), waiting);

    // Taking the commands each tick loses none.
    let mut commands = run.drain();
    for _ in 0..40 {
        commands.extend(run.tick(1.0));
        if run.phase() == BuildPhase::Done {
            break;
        }
    }
    commands.extend(run.drain());
    assert_eq!(run.phase(), BuildPhase::Done);
    assert_eq!(setblocks(&commands).len(), 300);
    assert_eq!(commands.last().map(String::as_str), Some(RESTORE));
}

#[test]
fn a_second_stop_press_within_two_seconds_stops_the_build() {
    let mut run = BuildRun::ready(&row_of(40), rate(4));
    run.start_and_clear();

    assert!(!run.session.press_stop(10.0, &mut run.outbox));
    assert!(run.row().confirm_stop);
    assert!(run.session.press_stop(11.5, &mut run.outbox));
    assert_eq!(run.phase(), BuildPhase::Stopped(BuildStopReason::Operator));
}

#[test]
fn a_stop_prompt_lapses_after_two_seconds() {
    let mut run = BuildRun::ready(&row_of(40), rate(4));
    run.start_and_clear();
    let start = run.time;

    assert!(!run.session.press_stop(start, &mut run.outbox));
    run.time = start + 2.5;
    run.session.tick(run.time, &mut run.outbox);
    assert!(!run.row().confirm_stop);
    assert!(!run.session.press_stop(run.time, &mut run.outbox));
    assert!(run.phase().is_running());
}

#[test]
fn the_restore_always_fits_in_a_full_outbox() {
    // 64 blocks would fill the whole outbox, leaving no room for the command that puts the game rule back.
    let mut run = BuildRun::ready(&row_of(64), rate(20));
    run.start_and_clear();
    run.drain();

    run.time += 10.0;
    run.session.tick(run.time, &mut run.outbox);
    assert!(run.outbox.free() >= 1);
    let mut commands = run.drain();
    for _ in 0..4 {
        commands.extend(run.tick(1.0));
    }

    assert_eq!(run.phase(), BuildPhase::Done);
    assert_eq!(setblocks(&commands).len(), 64);
    assert_eq!(commands.last().map(String::as_str), Some(RESTORE));
}

#[test]
fn an_assembly_job_is_refused_past_the_queue_bound_and_handed_back() {
    let piece = Pieces::layer_at(&["##"], [0, 64, 0]);
    let assemble = |serial| {
        BuildJob::Assemble(stratum_core::structs::build::AssembleRequest {
            serial,
            id: stratum_core::structs::library::StructureId(1),
            structure: std::sync::Arc::clone(&piece.structure),
            origin: [0, 64, 0],
            rotation: Rotation::R90,
        })
    };
    let parked = BuildOrderWorker::without_thread();
    for serial in 0..BuildOrderWorker::QUEUE as u64 {
        assert!(parked.submit(assemble(serial)).is_ok());
    }
    assert!(matches!(parked.submit(assemble(99)), Err(BuildJob::Assemble(back)) if back.serial == 99));

    // The job the worker does is the same as the one that gives the answer on this thread.
    let BuildAnswer::Assemble(done) = BuildOrderWorker::perform(assemble(5)) else {
        panic!("an assembly answers with a build");
    };
    assert_eq!(done.serial, 5);
    assert_eq!(done.build.unwrap().rotation, Rotation::R90);
}

#[test]
fn an_assembly_answer_that_does_not_stand_is_dropped() {
    use stratum_core::config::BuildConfig;
    use stratum_core::services::BuildSession;
    use stratum_core::structs::build::AssembleResult;
    use stratum_core::structs::library::StructureId;

    let piece = Pieces::layer_at(&["##"], [0, 64, 0]);
    let mut session = BuildSession::new(
        BuildConfig::default().kinds_capacity(),
        StructureId(1),
        "tower".to_string(),
        BuildSettings::default(),
    );
    session.start_serials(1 << 20);
    session.begin_assemble(BuildFixture::anchor(1), std::sync::Arc::clone(&piece.structure), [0, 64, 0], Rotation::R0);
    assert!(!session.wants_build());
    let job = session.take_job().expect("the assembly is the first job");
    let BuildAnswer::Assemble(done) = BuildOrderWorker::perform(job) else {
        panic!("an assembly answers with a build");
    };

    // An answer with another serial, as an older session would send, changes nothing.
    session.apply_answer(BuildAnswer::Assemble(AssembleResult { serial: 1, build: done.build.clone() }));
    assert!(session.take_probe().is_none());

    session.apply_answer(BuildAnswer::Assemble(done));
    assert!(session.take_probe().is_some());
}
