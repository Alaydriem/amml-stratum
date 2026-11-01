use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::{Duration, Instant};

use alaydriem_stratum::services::ManagerSession;
use alaydriem_stratum::types::PathList;
use amm_abi::types::AmmLogLevel;
use amm_client::test_support::FakeTable;
use amm_core::structs::descriptor::Capability;
use stratum_core::config::ImportSettings;
use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::ghost::GhostJobMode;
use stratum_core::structs::snapshot::ImportNotice;

use crate::support::{CoreSession, GhostScene, GhostSeen, Hud};

// Picker answers as the tick hands them over.
struct Picks;

impl Picks {
    fn pending(session: &CoreSession) -> usize {
        session.handle().with_state(|state| state.pending_imports()).unwrap()
    }

    fn take_request(session: &CoreSession) -> bool {
        session.handle().with_state(|state| state.take_picker_request()).unwrap()
    }

    // `paths` through the bound of one picker answer, then to the library
    fn picked(session: &CoreSession, paths: Vec<String>) -> Option<()> {
        let files = PathList::new(paths).into_paths();
        session.handle().with_state(|state| state.files_picked(files))
    }
}

#[test]
fn connect_places_no_menu_entry() {
    let _session = CoreSession::new();

    let entries = FakeTable::with(|state| state.menu_entries.clone());

    assert!(entries.is_empty());
}

#[test]
fn shutdown_saves_the_library_then_unregisters() {
    let mut session = CoreSession::new();
    session.shared().send(&mut vec![LibraryAction::CreateProject("Castle".to_owned())]);
    assert_eq!(session.tick(&CoreSession::in_world()), Some(()));

    session.destroy();

    let library = std::fs::read_to_string(session.data.path().join("library.hcl")).unwrap();
    assert!(library.contains("Castle"));
    assert_eq!(FakeTable::with(|state| state.unregistered.len()), 1);
}

#[test]
fn tick_in_and_out_of_world_is_ok() {
    let session = CoreSession::new();

    assert_eq!(session.tick(&CoreSession::in_world()), Some(()));
    assert_eq!(session.tick(&CoreSession::out_of_world()), Some(()));
    assert!(session.shared().snapshot().revision > 0);
}

#[test]
fn panic_under_the_tick_lock_fails_later_ticks_and_gives_the_ui_back() {
    let session = CoreSession::new();
    session.open();

    let panicked = catch_unwind(AssertUnwindSafe(|| session.handle().with_state(|_| panic!("tick"))));

    assert!(panicked.is_err());
    // the lock the panic poisoned turns later ticks into failures, logged once
    assert_eq!(session.tick(&CoreSession::in_world()), None);
    assert_eq!(session.tick(&CoreSession::in_world()), None);
    assert_eq!(CoreSession::logged("poisoned").len(), 1);
    // the first call that finds the lock poisoned closes the library and gives its leases back
    assert!(!session.shared().is_shown());
    assert_eq!(FakeTable::with(|state| state.released.clone()), [Capability::Input, Capability::Overlay]);
}

#[test]
fn picker_request_is_returned_once() {
    let session = CoreSession::new();
    session.shared().send(&mut vec![LibraryAction::RequestFilePicker(None)]);
    assert!(!Picks::take_request(&session));

    assert_eq!(session.tick(&CoreSession::in_world()), Some(()));

    assert!(Picks::take_request(&session));
    assert!(!Picks::take_request(&session));
}

#[test]
fn picked_paths_reach_the_import_worker() {
    let session = CoreSession::new();
    let first = session.data.path().join("a.mcstructure").to_string_lossy().into_owned();
    let second = session.data.path().join("b.mcstructure").to_string_lossy().into_owned();

    assert_eq!(Picks::picked(&session, vec![first, second]), Some(()));

    assert_eq!(Picks::pending(&session), 2);
}

#[test]
fn picked_paths_past_the_bound_are_left_out_and_the_queue_refuses_the_rest() {
    let session = CoreSession::new();
    let names: Vec<String> = (0..=PathList::MAX)
        .map(|n| session.data.path().join(format!("{n}.mcstructure")).to_string_lossy().into_owned())
        .collect();
    assert_eq!(names.len(), PathList::MAX + 1);

    assert_eq!(Picks::picked(&session, names), Some(()));

    let lines = CoreSession::logged("picked files past the import queue were left out");
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].level, AmmLogLevel::Warn as u8);
    // the worker takes the default queue of 64, plus at most a full result queue and one request in hand while
    // it runs, and refuses the rest, which the library shows as a full queue
    let queue = ImportSettings::default().queue;
    let accepted = Picks::pending(&session);
    assert!((queue..=2 * queue + 1).contains(&accepted), "accepted {accepted}");
    assert!(session.shared().snapshot().notices.iter().any(|notice| matches!(notice, ImportNotice::ImportQueueFull)));
}

#[test]
fn cancelled_picker_imports_nothing() {
    let session = CoreSession::new();

    assert_eq!(Picks::picked(&session, Vec::new()), Some(()));

    assert_eq!(Picks::pending(&session), 0);
}

#[test]
fn a_failed_import_is_logged_with_the_file_and_the_reason() {
    let session = CoreSession::new();
    let junk = session.data.path().join("junk.mcstructure");
    std::fs::write(&junk, b"not nbt").unwrap();
    assert_eq!(Picks::picked(&session, vec![junk.to_string_lossy().into_owned()]), Some(()));

    for _ in 0..500 {
        assert_eq!(session.tick(&CoreSession::in_world()), Some(()));
        if Picks::pending(&session) == 0 {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }

    let lines = CoreSession::logged("structure import failed");
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].level, AmmLogLevel::Warn as u8);
    assert!(lines[0].fields_json.contains("junk.mcstructure"));
    assert!(lines[0].fields_json.contains("reason"));
}

#[test]
fn a_panic_poisons_the_take_the_report_queue_and_the_state() {
    let session = CoreSession::new();
    let ghost = session.handle().ghost().clone();

    let take = catch_unwind(AssertUnwindSafe(|| {
        let _take = ghost.take();
        panic!("take");
    }));
    let report = catch_unwind(AssertUnwindSafe(|| ghost.report(false, |_| panic!("report"))));
    let changed = catch_unwind(AssertUnwindSafe(|| session.handle().with_state(|_| panic!("block changed"))));

    assert!(take.is_err() && report.is_err() && changed.is_err());
    assert!(ghost.take().is_none());
    assert_eq!(ghost.report(false, |report| GhostScene::write(report, &[], false)), None);
    assert_eq!(session.handle().with_state(|state| state.block_changed([1, 2, 3])), None);
}

#[test]
fn block_changed_reaches_the_controller_on_the_next_tick() {
    let (session, seen) = GhostScene::start();
    let slot = seen.full_job().slot;
    GhostScene::finish(&session, &seen, 20 * 24);

    assert_eq!(session.handle().with_state(|state| state.block_changed([10, 64, -20])), Some(()));
    assert!(GhostSeen::take(&session).jobs.is_empty());
    session.tick(&CoreSession::in_world());

    let rebuilt = GhostSeen::take(&session);
    assert_eq!(rebuilt.jobs.len(), 1);
    assert!(rebuilt.full_job().skip_matching);
    assert_eq!(rebuilt.full_job().slot, slot);
}

#[test]
fn a_second_take_before_the_tick_gets_the_draws_and_no_jobs() {
    let (session, seen) = GhostScene::start();
    let first = GhostScene::finish(&session, &seen, 480);
    assert_eq!(first.unscaled_draws().len(), 1);
    assert_eq!(first.miniature_draws().len(), 1);

    let second = GhostSeen::take(&session);

    assert!(second.taken);
    assert!(second.jobs.is_empty() && second.frees.is_empty() && !second.clear_all && second.palette.is_none());
    assert_eq!(second.draws, first.draws);
}

#[test]
fn a_job_carries_the_padded_grid_of_its_section() {
    let (_session, seen) = GhostScene::start();

    let job = seen.full_job().clone();
    // the marker at (10, 64, -20) lies in section (0, 4, -2); a new marker shows the whole build, so the job is
    // the whole section
    assert_eq!(job.origin, [0, 64, -32]);
    assert_eq!(job.dims, [16, 16, 16]);
    assert!(job.skip_matching);
    assert_eq!(seen.full_job_generation(), seen.generation);
    let cells = &job.cells;
    assert_eq!(cells.len(), 18 * 18 * 18);
    // the padded grid starts one cell below the origin, X fastest, then Z, then Y
    let at = |world: [i32; 3]| {
        let [x, y, z] = [0, 1, 2].map(|axis| (world[axis] - job.origin[axis] + 1) as usize);
        cells[(y * 18 + z) * 18 + x]
    };
    assert_eq!(at([10, 64, -20]), 0);
    assert_eq!(at([11, 64, -19]), 0);
    assert_eq!(at([12, 64, -20]), u16::MAX);
    assert_eq!(at([-1, 63, -33]), u16::MAX);
}

#[test]
fn the_take_carries_the_palette_and_opacity() {
    let session = CoreSession::new();
    GhostScene::import(&session);
    GhostScene::place(&session);
    let start = Instant::now();
    let mut palette = None;
    let mut opacity = 0.0;
    while palette.is_none() {
        assert!(start.elapsed() < Duration::from_secs(60), "no palette arrived");
        session.tick(&CoreSession::in_world());
        let seen = GhostSeen::take(&session);
        palette = palette.or(seen.palette);
        opacity = seen.opacity;
        std::thread::sleep(Duration::from_millis(2));
    }

    let palette = palette.unwrap();
    assert_eq!(palette.len(), 2);
    // a little-endian root compound that names the block
    assert_eq!(palette[0][0], 10);
    assert!(palette[0].windows(b"minecraft:stone".len()).any(|window| window == b"minecraft:stone"));
    assert!((opacity - 0.5).abs() < 1e-6);
}

#[test]
fn a_built_report_turns_into_a_draw_at_the_job_origin() {
    let (session, seen) = GhostScene::start();
    assert!(seen.draws.is_empty());
    let job = seen.full_job().clone();
    assert_eq!(job.mode, GhostJobMode::Replace);
    assert_eq!(job.slot_size, job.dims);

    // the mini ghost is one Append for its only section, placed relative to the build's lowest corner
    // (10, 64, -20) because the section starts at (0, 64, -32), and then one Finish
    let appends: Vec<_> =
        seen.jobs.iter().map(|(_, job)| job).filter(|candidate| candidate.mode == GhostJobMode::Append).collect();
    assert_eq!(appends.len(), 1);
    assert_eq!(appends[0].group_index, 0);
    assert_ne!(appends[0].group_serial, 0);
    assert_eq!(appends[0].group_offset, [-10, 0, -12]);
    assert_eq!(appends[0].origin, [10, 64, -20]);
    assert_eq!(appends[0].slot_size, [2, 2, 2]);
    assert!(seen.jobs.iter().all(|(_, candidate)| candidate.mode != GhostJobMode::Finish));

    assert_eq!(GhostScene::report_built(&session, &seen, 480), Some(()));
    session.tick(&CoreSession::in_world());
    let next = GhostSeen::take(&session);
    assert_eq!(next.jobs.len(), 1);
    let finish = &next.jobs[0].1;
    assert_eq!(finish.mode, GhostJobMode::Finish);
    assert_eq!(finish.group_serial, appends[0].group_serial);
    assert_eq!(finish.group_index, 1);
    assert_eq!(finish.slot, appends[0].slot);

    let drawn = GhostScene::finish(&session, &next, 480);
    let ghost = drawn.unscaled_draws();
    assert_eq!(ghost.len(), 1);
    assert_eq!((ghost[0].slot, ghost[0].origin, ghost[0].offset), (job.slot, job.origin, [0, 0, 0]));
    // the mini ghost scales about the build's lowest corner and ends on it: the 2-block build moves it 2 cells out on
    // X and Z
    let mini = drawn.miniature_draws();
    assert_eq!(mini.len(), 1);
    assert_eq!((mini[0].origin, mini[0].offset, mini[0].scale), ([10, 64, -20], [-2, 0, -2], 0.25));
}

// The render thread draws many frames per tick: the build answer comes in the first report, and the empty reports
// of the frames after it fill the queue past its capacity before the tick drains it.
#[test]
fn a_build_answer_followed_by_twenty_reports_is_drawn_after_the_drain() {
    let (session, seen) = GhostScene::start();
    let slot = seen.full_job().slot;
    assert_eq!(GhostScene::report_built(&session, &seen, 480), Some(()));
    for _ in 0..20 {
        assert_eq!(session.handle().ghost().report(false, |report| GhostScene::write(report, &[], false)), Some(()));
    }

    session.tick(&CoreSession::in_world());

    let drawn = GhostSeen::take(&session);
    let ghost = drawn.unscaled_draws();
    assert_eq!(ghost.len(), 1);
    assert_eq!(ghost[0].slot, slot);
    assert_eq!(drawn.jobs.len(), 1);
    assert_eq!(drawn.jobs[0].1.mode, GhostJobMode::Finish);
    let done = GhostScene::finish(&session, &drawn, 480);
    assert_eq!(done.miniature_draws().len(), 1);
}

#[test]
fn the_take_carries_the_world_epoch_which_moves_when_the_world_is_left() {
    let (session, seen) = GhostScene::start();
    let epoch = seen.world_epoch;
    session.tick(&CoreSession::in_world());
    let inside = GhostSeen::take(&session);
    assert_eq!(inside.world_epoch, epoch);
    assert!(inside.in_world);

    session.tick(&CoreSession::out_of_world());
    let outside = GhostSeen::take(&session);
    assert!(!outside.in_world);
    session.tick(&CoreSession::in_world());

    let back = GhostSeen::take(&session);
    assert_ne!(back.world_epoch, epoch);
    assert_ne!(back.world_epoch, outside.world_epoch);
    assert!(back.in_world);
}

#[test]
fn a_world_reset_report_brings_a_new_palette_and_a_clear() {
    let (session, seen) = GhostScene::start();
    GhostScene::report_built(&session, &seen, 480);
    session.tick(&CoreSession::in_world());
    let built = GhostSeen::take(&session);
    assert!(built.palette.is_none() && !built.clear_all);

    let reset = session.handle().ghost().report(true, |report| GhostScene::write(report, &[], true));
    assert_eq!(reset, Some(()));
    session.tick(&CoreSession::in_world());

    let fresh = GhostSeen::take(&session);
    assert!(fresh.clear_all);
    assert_eq!(fresh.palette.map(|entries| entries.len()), Some(2));
    assert_ne!(fresh.generation, seen.generation);
    assert!(!fresh.jobs.is_empty());
}

#[test]
fn the_take_carries_the_rotation_of_a_turned_marker() {
    let (session, seen) = GhostScene::start();
    assert_eq!(seen.rotation, 0);

    // Full mode, Left from Layer wraps to Easy place and Left again reaches Rotate, and Up turns the
    // marker a quarter turn clockwise
    FakeTable::push_event(Hud::press(ManagerSession::HUD_MODE_ACTION));
    FakeTable::push_event(Hud::press(ManagerSession::HUD_LEFT_ACTION));
    FakeTable::push_event(Hud::press(ManagerSession::HUD_LEFT_ACTION));
    FakeTable::push_event(Hud::press(ManagerSession::HUD_UP_ACTION));
    let start = Instant::now();
    loop {
        assert!(start.elapsed() < Duration::from_secs(60), "the turn did not reach the take");
        session.tick(&CoreSession::in_world());
        if GhostSeen::take(&session).rotation == 1 {
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}
