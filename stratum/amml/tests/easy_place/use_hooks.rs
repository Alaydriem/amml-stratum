use alaydriem_stratum::easy_place::UseHooks;
use stratum_core::structs::block::Face;
use stratum_core::structs::easy_place::{Placement, UseCommand};

use crate::support::{FakeUse, FakeUseCore};

const PLACE: Placement =
    Placement { spot: [0, 64, -2], neighbour: [0, 64, -3], face: Face::South, click: [0.5, 0.5, 1.0] };

#[allow(unsafe_code)]
fn tick(rig: &FakeUse, hooks: &UseHooks, core: &mut FakeUseCore, now: f64) {
    // SAFETY: the rig's fake client, player and region
    unsafe { hooks.tick(Some(core), &rig.frame(now), &mut FakeUse::palette_block) };
}

fn selects(log: &[String]) -> usize {
    log.iter().filter(|line| line.starts_with("select")).count()
}

#[test]
fn a_select_goes_through_the_hud_of_the_last_hud_tick() {
    let rig = FakeUse::new();
    let hooks = rig.hooks();
    let mut core = FakeUseCore::new(UseCommand::Select { slot: 2, place: PLACE });
    hooks.hud_tick(rig.hud, 1.0);
    // another screen controller goes away
    hooks.screen_gone(rig.hud + 0x100);

    tick(&rig, hooks, &mut core, 1.1);

    assert_eq!(selects(&FakeUse::log()), 1);
}

#[test]
fn after_the_held_hud_is_destroyed_a_select_does_not_call_it() {
    let rig = FakeUse::new();
    let hooks = rig.hooks();
    let mut core = FakeUseCore::new(UseCommand::Select { slot: 2, place: PLACE });
    hooks.hud_tick(rig.hud, 1.0);

    hooks.screen_gone(rig.hud);
    tick(&rig, hooks, &mut core, 1.1);

    let log = FakeUse::log();
    assert_eq!(selects(&log), 0, "{log:?}");
    assert!(log.contains(&"not_sent 0 64 -2".to_string()));
    assert_eq!(hooks.signals().hud().0, 0);
}

#[test]
fn a_hud_not_ticked_for_half_a_second_is_not_used() {
    let rig = FakeUse::new();
    let hooks = rig.hooks();
    let mut core = FakeUseCore::new(UseCommand::Select { slot: 2, place: PLACE });
    hooks.hud_tick(rig.hud, 1.0);

    tick(&rig, hooks, &mut core, 1.5);

    assert_eq!(selects(&FakeUse::log()), 0);
}

#[test]
#[allow(unsafe_code)]
fn the_executor_s_own_build_is_not_blocked_by_its_hook() {
    let rig = FakeUse::new();
    let hooks = rig.hooks();
    FakeUse::build_through(hooks);
    let mut core = FakeUseCore::new(UseCommand::Place(PLACE));

    tick(&rig, hooks, &mut core, 1.0);
    tick(&rig, hooks, &mut core, 1.3);

    let log = FakeUse::log();
    assert_eq!(log.iter().filter(|line| line.starts_with("build 0 64 -3")).count(), 2, "{log:?}");
    assert!(!log.contains(&"build skipped".to_string()));
    // the game's own build for this player is skipped while the hold blocks
    // SAFETY: the rig's fake game mode
    assert!(unsafe { hooks.blocks_game_build(rig.game_mode, false) });
}

#[test]
#[allow(unsafe_code)]
fn a_build_for_another_player_on_the_server_thread_is_never_blocked() {
    let rig = FakeUse::new();
    let hooks = rig.hooks();
    let mut core = FakeUseCore::new(UseCommand::Place(PLACE));
    tick(&rig, hooks, &mut core, 1.0);
    assert!(hooks.signals().holding() && hooks.signals().blocking());
    let server_player = FakeUse::game_mode_of(0x5000) as usize;

    let blocked = std::thread::spawn(move || {
        // SAFETY: a fake game mode of another player, leaked for the test process
        unsafe { hooks.blocks_game_build(server_player as *mut std::ffi::c_void, true) }
    })
    .join()
    .unwrap();

    assert!(!blocked);
}

#[test]
#[allow(unsafe_code)]
fn a_vanilla_hold_blocks_no_build() {
    let rig = FakeUse::new();
    let hooks = rig.hooks();
    let mut core = FakeUseCore::new(UseCommand::Vanilla);

    tick(&rig, hooks, &mut core, 1.0);

    // SAFETY: the rig's fake game mode
    assert!(!unsafe { hooks.blocks_game_build(rig.game_mode, false) });
}

#[test]
fn packets_come_to_the_request_of_the_driver() {
    let rig = FakeUse::new();
    let hooks = rig.hooks();
    let mut core = FakeUseCore::new(UseCommand::Pick { palette_id: FakeUse::PLANKS_PALETTE_ID, slot: 0 });
    tick(&rig, hooks, &mut core, 1.0);

    assert!(!hooks.on_container_open(1.1, FakeUse::open_packet(2, 0)));
    assert!(hooks.on_container_open(1.1, FakeUse::open_packet(2, -1)));
    assert!(hooks.on_container_close(1.2, FakeUse::close_packet(2)));
    hooks.on_response(FakeUse::response(alaydriem_stratum::easy_place::StackRequest::FIRST_REQUEST_ID, 1, None));

    assert_eq!(rig.slot(0), (FakeUse::dirt(), 10), "the refused pick is undone");
}

#[test]
fn a_reset_ends_the_hold_and_undoes_the_request_in_flight() {
    let rig = FakeUse::new();
    let hooks = rig.hooks();
    let mut core = FakeUseCore::new(UseCommand::Swap { from: 9, to: 0 });
    tick(&rig, hooks, &mut core, 1.0);

    hooks.reset();

    assert!(!hooks.signals().holding() && hooks.signals().player() == 0);
    assert_eq!(rig.slot(0), (FakeUse::dirt(), 10));
    assert!(!FakeUse::log().contains(&"released".to_string()), "a reset does not tell the core");
}
