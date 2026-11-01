use std::sync::atomic::Ordering;

use alaydriem_stratum::easy_place::{UseDriver, UseSignals};
use stratum_core::structs::block::Face;
use stratum_core::structs::easy_place::{GameMode, Placement, SwapAnswer, UseCommand};

use crate::support::fake_packet::{DELETE_CALLS, DESTROY_CALLS, READ_OK, SENT_IDS, STREAM_DTOR_CALLS};
use crate::support::fake_use::{ITEM_DTOR_CALLS, SELECT_TAKES};
use crate::support::{FakePacket, FakeUse, FakeUseCore};

const PLACE: Placement =
    Placement { spot: [0, 64, -2], neighbour: [0, 64, -3], face: Face::South, click: [0.5, 0.5, 1.0] };
const BUILD: &str = "build 0 64 -3 face=3 hand=0 sim=0";

fn lines(lines: &[&str]) -> Vec<String> {
    lines.iter().map(|line| line.to_string()).collect()
}

// one tick of a driver over the rig with `core`, at `now`
#[allow(unsafe_code)]
fn tick(rig: &FakeUse, driver: &mut UseDriver, signals: &UseSignals, core: &mut FakeUseCore, now: f64) {
    // SAFETY: the rig's fake client, player and region
    unsafe { driver.tick(signals, Some(core), &rig.frame(now), &mut FakeUse::palette_block) };
}

fn driver(rig: &FakeUse) -> UseDriver {
    UseDriver::new(&rig.packets, &rig.players, &rig.calls)
}

#[test]
fn a_use_call_reads_the_probe_the_spot_and_the_inventory_from_the_game() {
    let rig = FakeUse::new();
    let (mut driver, signals) = (driver(&rig), UseSignals::new());
    let mut core = FakeUseCore::new(UseCommand::Vanilla);

    tick(&rig, &mut driver, &signals, &mut core, 1.0);

    let probe = core.probe.expect("the core was asked");
    assert!(probe.pressed && probe.held);
    assert_eq!(probe.eye, [0.5, f64::from(65.62f32), 0.5]);
    let length = (0.25f64 + 1.0).sqrt();
    assert!((probe.dir[1] + 0.5 / length).abs() < 1e-6 && (probe.dir[2] + 1.0 / length).abs() < 1e-6);
    assert_eq!(probe.reach, 12.0);
    assert_eq!(probe.game_mode, GameMode::Creative);
    assert_eq!(probe.body, Some(([0, 64, 0], [0, 65, 0])));
    let hit = probe.hit.expect("the ray reaches the stone");
    assert_eq!((hit.cell, hit.face, hit.replaceable), ([0, 64, -3], Face::South, false));
    assert_eq!(probe.swap_answer, SwapAnswer::None);
    // only the stone north of the spot can be placed against
    assert_eq!(core.world.map(|world| world.against), Some([false, false, true, false, false, false]));
    assert_eq!(core.slots.len(), 36);
    assert_eq!((core.slots[0].1, core.slots[9].1, core.slots[1].1), (10, 64, 0));
}

#[test]
fn vanilla_leaves_the_use_to_the_game() {
    let rig = FakeUse::new();
    let (mut driver, signals) = (driver(&rig), UseSignals::new());
    let mut core = FakeUseCore::new(UseCommand::Vanilla);

    tick(&rig, &mut driver, &signals, &mut core, 1.0);

    assert_eq!(FakeUse::log(), lines(&["aim", "decide"]));
    assert!(signals.holding() && !signals.blocking());
}

#[test]
fn select_selects_through_the_hud_then_places() {
    let rig = FakeUse::new();
    let (mut driver, signals) = (driver(&rig), UseSignals::new());
    alaydriem_stratum::easy_place::UseExecutor::capture_hud(&signals, rig.hud, 0.9);
    let mut core = FakeUseCore::new(UseCommand::Select { slot: 3, place: PLACE });

    tick(&rig, &mut driver, &signals, &mut core, 1.0);

    assert_eq!(FakeUse::log(), lines(&["aim", "decide", "select 3", BUILD]));
    assert_eq!(rig.selected(), 3);
    assert!(signals.blocking());
}

#[test]
fn a_select_the_game_does_not_take_places_nothing_and_is_not_sent() {
    SELECT_TAKES.store(false, Ordering::SeqCst);
    let rig = FakeUse::new();
    let (mut driver, signals) = (driver(&rig), UseSignals::new());
    alaydriem_stratum::easy_place::UseExecutor::capture_hud(&signals, rig.hud, 0.9);
    let mut core = FakeUseCore::new(UseCommand::Select { slot: 3, place: PLACE });

    tick(&rig, &mut driver, &signals, &mut core, 1.0);

    assert_eq!(FakeUse::log(), lines(&["aim", "decide", "select 3", "not_sent 0 64 -2"]));
}

#[test]
fn swap_sends_the_inventory_open_and_the_request_then_predicts() {
    let rig = FakeUse::new();
    let (mut driver, signals) = (driver(&rig), UseSignals::new());
    let mut core = FakeUseCore::new(UseCommand::Swap { from: 9, to: 0 });

    tick(&rig, &mut driver, &signals, &mut core, 1.0);

    assert_eq!(FakeUse::log(), lines(&["aim", "decide", "swap_slots 0 9"]));
    assert_eq!(*SENT_IDS.lock().unwrap(), vec![33, 147]);
}

#[test]
fn pick_asks_the_palette_and_the_block_s_item_then_sends_and_predicts() {
    let rig = FakeUse::new();
    let (mut driver, signals) = (driver(&rig), UseSignals::new());
    let mut core = FakeUseCore::new(UseCommand::Pick { palette_id: FakeUse::PLANKS_PALETTE_ID, slot: 0 });

    tick(&rig, &mut driver, &signals, &mut core, 1.0);

    assert_eq!(FakeUse::log(), lines(&["aim", "decide", "copy_stack", "set_item 0"]));
    assert_eq!(*SENT_IDS.lock().unwrap(), vec![33, 147]);
    assert_eq!(rig.slot(0), (FakeUse::planks(), 64));
    assert_eq!(FakeUse::count(&ITEM_DTOR_CALLS), 1, "the block's item instance is destroyed once");
}

#[test]
fn place_builds_on_the_neighbour_with_the_hit_set_and_puts_the_hit_back() {
    let rig = FakeUse::new();
    let (mut driver, signals) = (driver(&rig), UseSignals::new());
    let mut core = FakeUseCore::new(UseCommand::Place(PLACE));

    tick(&rig, &mut driver, &signals, &mut core, 1.0);

    assert_eq!(FakeUse::log(), lines(&["aim", "decide", BUILD]));
    // SAFETY: the rig's fake hit result
    #[allow(unsafe_code)]
    let hit = unsafe { &*rig.hit };
    assert_eq!((hit.kind, hit.facing, hit.block.x, hit.pos), (0, 0, 0, [0.0; 3]));
    assert!(!signals.own_build(), "the own-build guard is down after the build");
}

#[test]
fn a_refused_build_is_not_sent() {
    crate::support::fake_use::BUILD_OK.store(false, Ordering::SeqCst);
    let rig = FakeUse::new();
    let (mut driver, signals) = (driver(&rig), UseSignals::new());
    let mut core = FakeUseCore::new(UseCommand::Place(PLACE));

    tick(&rig, &mut driver, &signals, &mut core, 1.0);

    assert_eq!(FakeUse::log(), lines(&["aim", "decide", BUILD, "not_sent 0 64 -2"]));
}

#[test]
fn a_swap_whose_packet_read_fails_is_not_sent_released_once_and_answered_refused() {
    READ_OK.store(false, Ordering::SeqCst);
    let rig = FakeUse::new();
    let (mut driver, signals) = (driver(&rig), UseSignals::new());
    let mut core = FakeUseCore::new(UseCommand::Swap { from: 9, to: 0 });

    tick(&rig, &mut driver, &signals, &mut core, 1.0);

    // no inventory open, no request and no prediction
    assert!(SENT_IDS.lock().unwrap().is_empty());
    assert_eq!(FakeUse::log(), lines(&["aim", "decide"]));
    assert_eq!(FakePacket::count(&STREAM_DTOR_CALLS), 1);
    assert_eq!(FakePacket::count(&DESTROY_CALLS), 1);
    assert_eq!(FakePacket::count(&DELETE_CALLS), 1);
    assert_eq!(driver.request().answer(), SwapAnswer::Refused);
    assert_eq!(driver.request().awaited(), 0);

    // the next call goes on as usual
    tick(&rig, &mut driver, &signals, &mut core, 2.0);
    assert_eq!(core.probe.map(|probe| probe.swap_answer), Some(SwapAnswer::Refused));
}

#[test]
fn a_held_key_calls_once_per_build_delay_or_on_the_answer() {
    let rig = FakeUse::new();
    let (mut driver, signals) = (driver(&rig), UseSignals::new());
    let mut core = FakeUseCore::new(UseCommand::Swap { from: 9, to: 0 });
    tick(&rig, &mut driver, &signals, &mut core, 1.0);
    core.command = UseCommand::Vanilla;
    let id = driver.request().awaited();

    tick(&rig, &mut driver, &signals, &mut core, 1.1);
    assert_eq!(FakeUse::log().iter().filter(|line| *line == "aim").count(), 1, "the 200 ms delay has not passed");

    driver.request().on_response(FakeUse::response(id, 0, None));
    tick(&rig, &mut driver, &signals, &mut core, 1.15);
    assert_eq!(FakeUse::log().iter().filter(|line| *line == "aim").count(), 2, "the answer came");
    assert!(!core.probe.unwrap().pressed);
    assert_eq!(core.probe.unwrap().swap_answer, SwapAnswer::Accepted);
}

#[test]
fn key_up_tells_the_core_once() {
    let rig = FakeUse::new();
    let (mut driver, signals) = (driver(&rig), UseSignals::new());
    let mut core = FakeUseCore::new(UseCommand::Place(PLACE));
    tick(&rig, &mut driver, &signals, &mut core, 1.0);
    let mut frame = rig.frame(1.5);
    frame.key_held = false;

    for _ in 0..2 {
        // SAFETY: the rig's fake objects
        #[allow(unsafe_code)]
        unsafe {
            driver.tick(&signals, Some(&mut core), &frame, &mut FakeUse::palette_block)
        };
    }

    assert_eq!(FakeUse::log().iter().filter(|line| *line == "released").count(), 1);
    assert!(!signals.holding() && signals.player() == 0);
}
