use std::sync::{Arc, Mutex};

use alaydriem_stratum::easy_place::{UseCycle, UseDriver, UseFlags};
use alaydriem_stratum::traits::UseCoreHolder;
use amml_api::amml_abi::{BuildBlock, ClientTick, InventoryRequestDone, UsePressed};
use amml_api::testing::{FakeClient, FakeGame, FakeInventory, FakeItem};
use amml_api::{
    BlockHandle, Cancel, GameType, HookOrder, InventoryMove, InventoryRequestDoneExt, Listener, RequestOptions, Screen,
    Vec3,
};
use stratum_core::structs::block::Face;
use stratum_core::structs::easy_place::{Placement, SwapAnswer, UseCommand};

use crate::support::{FakeCoreHolder, FakeUseCore, StratumRig};

const STONE: FakeItem = FakeItem { type_name: "minecraft:stone", count: 64, ..FakeItem::EMPTY };

// a use driver run from the fake client's ticks and use presses with the real use key held, through `UseCycle` as the
// runtime runs it: decided under the locks, carried out with none held. The request answers go back through
// `InventoryRequestDone`; each `BuildBlock` the fake raises inside `build_at` records whether both locks were free on
// that stack
struct DriverRig {
    game: FakeGame,
    core: Arc<Mutex<FakeCoreHolder>>,
    // per `BuildBlock` on the stack of a build: the core and the driver lock could both be taken
    free_on_build: Arc<Mutex<Vec<bool>>>,
    _listeners: Vec<Listener>,
}

impl DriverRig {
    fn new(game_type: GameType, command: UseCommand, slots: [Option<FakeItem>; 36]) -> Self {
        let game = FakeGame::new();
        game.offer::<ClientTick>();
        game.offer::<InventoryRequestDone>();
        game.offer::<UsePressed>();
        game.offer::<BuildBlock>();
        for area in StratumRig::areas() {
            game.offer_area(area);
        }
        game.open_gate();
        game.set_epoch(1);
        game.set_client(FakeClient {
            game_type: Some(game_type),
            top_screen: Screen::Hud,
            head_position: Some(Vec3 { x: 0.5, y: 66.6, z: -1.5 }),
            view_vector: Some(Vec3 { x: 0.0, y: -1.0, z: 0.0 }),
            ..StratumRig::client()
        });
        game.set_inventory(FakeInventory { selected_slot: Some(0), slots, ..FakeInventory::default() });
        let core = Arc::new(Mutex::new(FakeCoreHolder { core: FakeUseCore::new(command) }));
        let driver = Arc::new(Mutex::new(UseDriver::new()));
        let flags = Arc::new(UseFlags::new());
        let pick = |command: &UseCommand| {
            matches!(command, UseCommand::Pick { .. }).then(|| BlockHandle::for_test(StratumRig::STONE))
        };
        let events = game.ctx().events();
        let (tick_core, tick_driver, tick_flags) = (Arc::clone(&core), Arc::clone(&driver), Arc::clone(&flags));
        let tick = events
            .listen::<ClientTick>(HookOrder::READ_AFTER_WRITERS, move |_, at| {
                let Some(client) = at.client() else { return };
                let player = client.local_player();
                let (command, counts) = {
                    let mut core = tick_core.lock().unwrap();
                    let mut driver = tick_driver.lock().unwrap();
                    (driver.tick(&tick_flags, core.use_core(), player.as_ref(), true, 1.0), driver.counts())
                };
                let (Some(command), Some(player)) = (command, player.as_ref()) else { return };
                UseCycle::carry_out(&tick_core, &tick_driver, player, &client.hud(), &command, pick(&command), &counts);
            })
            .unwrap();
        let (press_core, press_driver) = (Arc::clone(&core), Arc::clone(&driver));
        let press = events
            .listen::<UsePressed>(HookOrder::WRITE, move |event, at| {
                let Some(client) = at.client() else { return };
                let player = client.local_player();
                let hud = client.hud();
                if UseCycle::press(&press_core, &press_driver, &flags, player.as_ref(), &hud, true, 1.0, pick) {
                    event.cancel();
                }
            })
            .unwrap();
        let free_on_build = Arc::new(Mutex::new(Vec::new()));
        let (build_core, build_driver, build_seen) =
            (Arc::clone(&core), Arc::clone(&driver), Arc::clone(&free_on_build));
        let build = events
            .listen::<BuildBlock>(HookOrder::WRITE, move |_, _| {
                let free = build_core.try_lock().is_ok() && build_driver.try_lock().is_ok();
                build_seen.lock().unwrap().push(free);
            })
            .unwrap();
        let done_driver = Arc::clone(&driver);
        let done = events
            .listen::<InventoryRequestDone>(HookOrder::NORMAL, move |event, _| {
                done_driver.lock().unwrap().request_done(event.id(), event.outcome());
            })
            .unwrap();
        Self { game, core, free_on_build, _listeners: vec![tick, press, build, done] }
    }

    fn tick(&self) {
        self.game.raise_with_client(&mut ClientTick { header: FakeGame::header::<ClientTick>() });
    }
}

fn slots_with(index: usize, item: FakeItem) -> [Option<FakeItem>; 36] {
    let mut slots: [Option<FakeItem>; 36] = std::array::from_fn(|_| None);
    slots[index] = Some(item);
    slots
}

#[test]
fn survival_moves_the_stack_into_an_empty_hotbar_slot_and_the_next_call_sees_the_answer() {
    let rig = DriverRig::new(GameType::Survival, UseCommand::Swap { from: 12, to: 0 }, slots_with(12, STONE));

    rig.tick();

    let requests = rig.game.requests();
    assert_eq!(requests.len(), 1);
    let (id, moves, options) = requests[0].clone();
    assert_eq!(moves, vec![InventoryMove::Place { from: 12, to: 0, count: 64 }]);
    assert_eq!(options, RequestOptions { hidden_open: true, predict: true });
    assert_eq!(rig.core.lock().unwrap().core.slots[12], (b"minecraft:stone".to_vec(), 64));

    rig.game.answer(id, 1);
    rig.game.frame();
    rig.tick();

    assert_eq!(rig.core.lock().unwrap().core.probe.map(|probe| probe.swap_answer), Some(SwapAnswer::Accepted));
}

#[test]
fn survival_swaps_two_stacks_when_the_hotbar_slot_holds_one() {
    let mut slots = slots_with(12, STONE);
    slots[0] = Some(FakeItem { type_name: "minecraft:dirt", count: 3, ..FakeItem::EMPTY });
    let rig = DriverRig::new(GameType::Survival, UseCommand::Swap { from: 12, to: 0 }, slots);

    rig.tick();

    let requests = rig.game.requests();
    assert_eq!(requests[0].1, vec![InventoryMove::Swap { a: 12, b: 0 }]);
}

#[test]
fn creative_picks_the_palette_block_into_the_hotbar_slot() {
    let rig = DriverRig::new(GameType::Creative, UseCommand::Pick { palette_id: 0, slot: 3 }, slots_with(12, STONE));

    rig.tick();

    let picks = rig.game.picks();
    assert_eq!(picks.len(), 1);
    assert_eq!((picks[0].1, picks[0].2), (BlockHandle::for_test(StratumRig::STONE), 3));
}

#[test]
fn a_place_builds_against_the_neighbour_face_at_the_click_point() {
    let place = Placement { spot: [0, 65, -2], neighbour: [0, 64, -2], face: Face::Up, click: [0.5, 1.0, 0.5] };
    let rig = DriverRig::new(GameType::Creative, UseCommand::Place(place), slots_with(0, STONE));

    rig.tick();

    assert_eq!(rig.game.builds(), vec!["build_at 0,64,-2 face=up".to_string()]);
    assert!(rig.core.lock().unwrap().core.log.iter().all(|line| !line.starts_with("not_sent")));
}

#[test]
fn a_select_the_game_does_not_take_is_reported_not_sent() {
    let place = Placement { spot: [0, 65, -2], neighbour: [0, 64, -2], face: Face::Up, click: [0.5, 1.0, 0.5] };
    // slot 9 is not a hotbar slot, so the select is not made and nothing is built
    let rig = DriverRig::new(GameType::Creative, UseCommand::Select { slot: 9, place }, slots_with(0, STONE));

    rig.tick();

    assert!(rig.game.builds().is_empty());
    assert!(rig.core.lock().unwrap().core.log.iter().any(|line| line == "not_sent 0 65 -2"));
}

#[test]
fn a_press_that_places_builds_with_no_lock_held_on_the_build_stack_and_cancels_the_game_use() {
    let place = Placement { spot: [0, 65, -2], neighbour: [0, 64, -2], face: Face::Up, click: [0.5, 1.0, 0.5] };
    let rig = DriverRig::new(GameType::Creative, UseCommand::Place(place), slots_with(0, STONE));

    let cancelled = rig.game.raise_use_pressed();

    assert!(cancelled);
    assert_eq!(rig.game.builds(), vec!["build_at 0,64,-2 face=up".to_string()]);
    assert_eq!(*rig.free_on_build.lock().unwrap(), vec![true], "BuildBlock on the build stack takes both locks");
}

#[test]
fn a_tick_that_places_builds_with_no_lock_held_on_the_build_stack() {
    let place = Placement { spot: [0, 65, -2], neighbour: [0, 64, -2], face: Face::Up, click: [0.5, 1.0, 0.5] };
    let rig = DriverRig::new(GameType::Creative, UseCommand::Place(place), slots_with(0, STONE));

    rig.tick();

    assert_eq!(*rig.free_on_build.lock().unwrap(), vec![true]);
}

#[test]
fn a_press_while_the_core_is_held_goes_to_the_game() {
    let place = Placement { spot: [0, 65, -2], neighbour: [0, 64, -2], face: Face::Up, click: [0.5, 1.0, 0.5] };
    let rig = DriverRig::new(GameType::Creative, UseCommand::Place(place), slots_with(0, STONE));
    let held = rig.core.lock().unwrap();

    let cancelled = rig.game.raise_use_pressed();

    drop(held);
    assert!(!cancelled);
    assert!(rig.game.builds().is_empty());
}

#[test]
fn a_build_delay_the_loader_cannot_read_falls_back_to_the_game_default_not_to_none() {
    let place = Placement { spot: [0, 65, -2], neighbour: [0, 64, -2], face: Face::Up, click: [0.5, 1.0, 0.5] };
    let rig = DriverRig::new(GameType::Creative, UseCommand::Place(place), slots_with(0, STONE));
    rig.game.set_client(FakeClient {
        game_type: Some(GameType::Creative),
        top_screen: Screen::Hud,
        build_delay: None,
        ..StratumRig::client()
    });

    rig.tick();
    rig.tick();

    assert_eq!(rig.game.builds().len(), 1, "the second tick at the same time is not due");
}

#[test]
fn with_no_block_on_the_walk_the_game_hit_result_is_the_hit() {
    use amml_api::{BlockHit, BlockPos, Face as ApiFace, HitResult};
    let rig = DriverRig::new(GameType::Creative, UseCommand::Vanilla, slots_with(0, STONE));
    let hit =
        BlockHit { pos: BlockPos { x: 0, y: 64, z: -2 }, point: Vec3 { x: 0.5, y: 65.0, z: -1.5 }, face: ApiFace::Up };
    rig.game.set_client(FakeClient {
        game_type: Some(GameType::Creative),
        top_screen: Screen::Hud,
        head_position: Some(Vec3 { x: 0.5, y: 66.6, z: -1.5 }),
        view_vector: Some(Vec3 { x: 0.0, y: -1.0, z: 0.0 }),
        hit_result: Some(HitResult::Block(hit)),
        ..StratumRig::client()
    });

    rig.tick();

    let probe = rig.core.lock().unwrap().core.probe.unwrap();
    let found = probe.hit.expect("the game's hit is used");
    assert_eq!((found.cell, found.face), ([0, 64, -2], Face::Up));
    assert!((found.distance - 1.6).abs() < 1e-3, "{}", found.distance);
    assert!(found.replaceable, "air at the hit cell can be built over");
}
