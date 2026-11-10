use std::sync::Arc;

use stratum_core::config::EasyPlaceConfig;
use stratum_core::services::{EasyPlaceService, GhostAssembler, ItemRules, PlaceNeeds};
use stratum_core::structs::block::Face;
use stratum_core::structs::easy_place::{
    EasyPlaceNotice, GameMode, InventorySlot, PlaceNeed, Placement, SpotWorld, SwapAnswer, UseCommand, UseHit, UseProbe,
};
use stratum_core::structs::ghost::{GhostBuild, GhostView, PlacedStructure};
use stratum_core::structs::marker::Rotation;
use stratum_core::structs::structure::StateValue;

use crate::support::{BlockFactory, GhostFixture};

const STONE: &[u8] = b"minecraft:stone";
const DIRT: &[u8] = b"minecraft:dirt";
const EMPTY: &[u8] = b"";

// Index of each face in `SpotWorld::against`.
const DOWN: usize = 0;
const UP: usize = 1;
const EAST: usize = 5;

// A service with a build, its needs and the face layout the shim would report.
struct Rig {
    service: EasyPlaceService,
    build: Option<GhostBuild>,
    needs: PlaceNeeds,
    against: [bool; 6],
}

impl Rig {
    fn new(build: GhostBuild) -> Self {
        Self::with_config(build, EasyPlaceConfig::default())
    }

    fn with_config(build: GhostBuild, config: EasyPlaceConfig) -> Self {
        let needs = PlaceNeeds::new(&build.palette, &ItemRules::new());
        let mut against = [false; 6];
        against[DOWN] = true;
        Self { service: EasyPlaceService::new(config), build: Some(build), needs, against }
    }

    fn config(edit: impl FnOnce(&mut EasyPlaceConfig)) -> EasyPlaceConfig {
        let mut config = EasyPlaceConfig::default();
        edit(&mut config);
        config
    }

    // One use call: aim, read the world when asked, decide.
    fn call(&mut self, probe: &UseProbe, inventory: &[InventorySlot<'_>], selected: u8) -> UseCommand {
        let built = self.build.as_ref().map(|build| (build, GhostView::Whole, &self.needs));
        let spot = self.service.aim(probe, built);
        let world = spot.map(|_| SpotWorld { against: self.against });
        self.service.decide(probe, world.as_ref(), inventory, selected)
    }

    // The item name of the first palette entry that is a target.
    fn item(&self) -> String {
        (0..8)
            .find_map(|id| match self.needs.need(id) {
                PlaceNeed::Place { item, .. } => Some(item.0.clone()),
                PlaceNeed::NotTarget => None,
            })
            .unwrap()
    }
}

struct Probes;

impl Probes {
    // Looks straight down at the cell column `x`, z 0 from five blocks above.
    fn above(x: i32, time_seconds: f64) -> UseProbe {
        UseProbe {
            time_seconds,
            pressed: false,
            held: true,
            eye: [f64::from(x) + 0.5, 5.5, 0.5],
            dir: [0.0, -1.0, 0.0],
            reach: 8.0,
            game_mode: GameMode::Survival,
            hit: None,
            body: None,
            swap_answer: SwapAnswer::None,
        }
    }

    fn looking_up(time_seconds: f64) -> UseProbe {
        UseProbe { dir: [0.0, 1.0, 0.0], ..Self::above(0, time_seconds) }
    }

    fn answer(mut probe: UseProbe, answer: SwapAnswer) -> UseProbe {
        probe.swap_answer = answer;
        probe
    }
}

struct Inventories;

impl Inventories {
    // 36 slots, empty except for the listed `(slot, item)` pairs which hold one item each.
    fn with<'a>(stacks: &[(usize, &'a [u8])]) -> Vec<InventorySlot<'a>> {
        let mut slots = vec![InventorySlot { item: EMPTY, count: 0 }; 36];
        for (slot, item) in stacks {
            slots[*slot] = InventorySlot { item, count: 1 };
        }
        slots
    }

    fn full_hotbar<'a>() -> Vec<InventorySlot<'a>> {
        let stacks: Vec<(usize, &[u8])> = (0..9).map(|slot| (slot, DIRT)).collect();
        Self::with(&stacks)
    }
}

struct Spots;

impl Spots {
    fn stone() -> GhostBuild {
        GhostFixture::blocks(&[([0, 0, 0], [0, 0, 0])])
    }

    fn row(count: i32) -> GhostBuild {
        GhostFixture::blocks(&[([0, 0, 0], [count - 1, 0, 0])])
    }

    fn single(name: &str) -> GhostBuild {
        let structure = GhostFixture::structure([1, 1, 1], &[name]);
        let placed = [PlacedStructure { structure: Arc::clone(&structure), origin: [0, 0, 0] }];
        GhostAssembler::build(&placed, &[structure], [0, 0, 0], Rotation::R0, 1, None).unwrap()
    }

    // Two cells side by side holding different blocks.
    fn two_kinds() -> GhostBuild {
        let structure = GhostFixture::structure([2, 1, 1], &["stone", "dirt"]);
        let placed = [PlacedStructure { structure: Arc::clone(&structure), origin: [0, 0, 0] }];
        GhostAssembler::build(&placed, &[structure], [0, 0, 0], Rotation::R0, 1, None).unwrap()
    }

    // The placement against the block below the spot at `x`.
    fn on_floor(x: i32) -> Placement {
        Placement { spot: [x, 0, 0], neighbour: [x, -1, 0], face: Face::Up, click: [0.5, 1.0, 0.5] }
    }
}

#[test]
fn a_held_block_is_placed() {
    let mut rig = Rig::new(Spots::stone());
    let inventory = Inventories::with(&[(3, STONE)]);
    let command = rig.call(&Probes::above(0, 0.0), &inventory, 3);
    assert_eq!(command, UseCommand::Place(Spots::on_floor(0)));
    assert_eq!(rig.service.telemetry().placed, 1);
}

#[test]
fn a_hotbar_block_is_selected_then_placed() {
    let mut rig = Rig::new(Spots::stone());
    let inventory = Inventories::with(&[(5, STONE), (7, STONE)]);
    let command = rig.call(&Probes::above(0, 0.0), &inventory, 3);
    assert_eq!(command, UseCommand::Select { slot: 5, place: Spots::on_floor(0) });
}

#[test]
fn a_main_inventory_block_is_swapped_and_placed_when_the_server_accepts() {
    let mut rig = Rig::new(Spots::stone());
    let before = Inventories::with(&[(20, STONE)]);
    assert_eq!(rig.call(&Probes::above(0, 0.0), &before, 3), UseCommand::Swap { from: 20, to: 3 });
    assert_eq!(rig.call(&Probes::above(0, 0.05), &before, 3), UseCommand::Block);
    let after = Inventories::with(&[(3, STONE)]);
    let accepted = Probes::answer(Probes::above(0, 0.06), SwapAnswer::Accepted);
    assert_eq!(rig.call(&accepted, &after, 3), UseCommand::Place(Spots::on_floor(0)));
}

#[test]
fn a_refused_swap_is_not_tried_again_in_the_hold() {
    let mut rig = Rig::new(Spots::stone());
    let inventory = Inventories::with(&[(20, STONE)]);
    assert_eq!(rig.call(&Probes::above(0, 0.0), &inventory, 3), UseCommand::Swap { from: 20, to: 3 });
    let refused = Probes::answer(Probes::above(0, 0.05), SwapAnswer::Refused);
    assert_eq!(rig.call(&refused, &inventory, 3), UseCommand::Block);
    assert_eq!(rig.call(&Probes::above(0, 0.3), &inventory, 3), UseCommand::Block);
    assert_eq!(rig.call(&Probes::above(0, 0.6), &inventory, 3), UseCommand::Block);
}

#[test]
fn a_swap_with_no_answer_times_out() {
    let mut rig = Rig::new(Spots::stone());
    let inventory = Inventories::with(&[(20, STONE)]);
    assert_eq!(rig.call(&Probes::above(0, 0.0), &inventory, 3), UseCommand::Swap { from: 20, to: 3 });
    assert_eq!(rig.call(&Probes::above(0, 0.4), &inventory, 3), UseCommand::Block);
    assert_eq!(rig.service.tried_len(), 0);
    assert_eq!(rig.call(&Probes::above(0, 0.5), &inventory, 3), UseCommand::Block);
    assert_eq!(rig.service.tried_len(), 1);
    assert_eq!(rig.call(&Probes::above(0, 0.7), &inventory, 3), UseCommand::Block);
}

#[test]
fn a_swap_moves_both_stacks_and_loses_nothing() {
    let mut rig = Rig::new(Spots::stone());
    let inventory = Inventories::with(&[(3, DIRT), (20, STONE)]);
    let command = rig.call(&Probes::above(0, 0.0), &inventory, 3);
    assert_eq!(command, UseCommand::Swap { from: 20, to: 3 });
}

#[test]
fn a_missing_block_in_survival_cancels_with_one_notice() {
    let mut rig = Rig::new(GhostFixture::blocks(&[([0, 0, 0], [0, 0, 0]), ([3, 0, 0], [3, 0, 0])]));
    let inventory = Inventories::with(&[(0, DIRT)]);
    assert_eq!(rig.call(&Probes::above(0, 0.0), &inventory, 0), UseCommand::Block);
    assert_eq!(rig.service.take_notice(), Some(EasyPlaceNotice::Missing { palette_id: 0 }));
    assert_eq!(rig.call(&Probes::above(3, 0.1), &inventory, 0), UseCommand::Block);
    assert_eq!(rig.service.take_notice(), None);
    assert_eq!(rig.service.telemetry().cancelled_missing, 2);
    rig.service.end_hold();
    assert_eq!(rig.call(&Probes::above(0, 1.0), &inventory, 0), UseCommand::Block);
    assert_eq!(rig.service.take_notice(), Some(EasyPlaceNotice::Missing { palette_id: 0 }));
}

#[test]
fn the_notice_list_holds_at_most_its_capacity() {
    let config = Rig::config(|config| config.tried_capacity = 1);
    let mut rig = Rig::with_config(Spots::two_kinds(), config);
    let inventory = Inventories::with(&[]);
    let mut ids = Vec::new();
    for x in [0, 1] {
        assert_eq!(rig.call(&Probes::above(x, 0.0), &inventory, 0), UseCommand::Block);
        let Some(EasyPlaceNotice::Missing { palette_id }) = rig.service.take_notice() else {
            panic!("no missing notice at x {x}");
        };
        ids.push(palette_id);
    }
    assert_ne!(ids[0], ids[1]);
    // The second id evicted the first, so the first id is named again.
    assert_eq!(rig.call(&Probes::above(0, 0.5), &inventory, 0), UseCommand::Block);
    assert_eq!(rig.service.take_notice(), Some(EasyPlaceNotice::Missing { palette_id: ids[0] }));
    assert_eq!(rig.service.tried_len(), 1);
}

#[test]
fn creative_pick_uses_an_empty_hotbar_slot() {
    let mut rig = Rig::new(Spots::stone());
    let inventory = Inventories::with(&[(0, DIRT), (1, DIRT)]);
    let probe = UseProbe { game_mode: GameMode::Creative, ..Probes::above(0, 0.0) };
    assert_eq!(rig.call(&probe, &inventory, 0), UseCommand::Pick { palette_id: 0, slot: 2 });
    let waiting = UseProbe { game_mode: GameMode::Creative, ..Probes::above(0, 0.1) };
    assert_eq!(rig.call(&waiting, &inventory, 0), UseCommand::Block);
    assert_eq!(rig.service.take_notice(), None);
}

#[test]
fn creative_pick_with_a_full_hotbar_names_the_selected_slot() {
    let mut rig = Rig::new(Spots::stone());
    let inventory = Inventories::full_hotbar();
    let probe = UseProbe { game_mode: GameMode::Creative, ..Probes::above(0, 0.0) };
    assert_eq!(rig.call(&probe, &inventory, 4), UseCommand::Pick { palette_id: 0, slot: 4 });
}

#[test]
fn no_visible_neighbour_cancels_with_a_notice() {
    let mut rig = Rig::new(Spots::stone());
    rig.against = [false; 6];
    rig.against[UP] = true;
    let inventory = Inventories::with(&[(0, STONE)]);
    assert_eq!(rig.call(&Probes::above(0, 0.0), &inventory, 0), UseCommand::Block);
    assert_eq!(rig.service.take_notice(), Some(EasyPlaceNotice::NoNeighbour));
    assert_eq!(rig.service.telemetry().cancelled_no_neighbour, 1);
    assert_eq!(rig.service.tried_len(), 1);
    assert_eq!(rig.call(&Probes::above(0, 0.2), &inventory, 0), UseCommand::Block);
    assert_eq!(rig.service.take_notice(), None);
}

#[test]
fn the_neighbour_nearest_the_ray_is_chosen() {
    let mut rig = Rig::new(Spots::stone());
    rig.against = [true; 6];
    let inventory = Inventories::with(&[(0, STONE)]);
    let mut probe = Probes::above(0, 0.0);
    probe.eye = [-4.5, 0.9, 0.5];
    probe.dir = [1.0, -0.1, 0.0];
    let command = rig.call(&probe, &inventory, 0);
    let expected = Placement { spot: [0, 0, 0], neighbour: [1, 0, 0], face: Face::West, click: [0.0, 0.5, 0.5] };
    assert_eq!(command, UseCommand::Place(expected));
}

#[test]
fn a_wall_torch_needs_a_side_face() {
    let mut rig = Rig::new(Spots::single("wall_torch"));
    let item = rig.item();
    let inventory = Inventories::with(&[(0, item.as_bytes())]);
    assert_eq!(rig.call(&Probes::above(0, 0.0), &inventory, 0), UseCommand::Block);
    assert_eq!(rig.service.take_notice(), Some(EasyPlaceNotice::NoNeighbour));
    rig.service.end_hold();
    rig.against[EAST] = true;
    let expected = Placement { spot: [0, 0, 0], neighbour: [1, 0, 0], face: Face::West, click: [0.0, 0.5, 0.5] };
    assert_eq!(rig.call(&Probes::above(0, 1.0), &inventory, 0), UseCommand::Place(expected));
}

#[test]
fn a_floor_torch_needs_the_block_below() {
    let mut rig = Rig::new(Spots::single("torch"));
    let item = rig.item();
    let inventory = Inventories::with(&[(0, item.as_bytes())]);
    rig.against = [false; 6];
    rig.against[EAST] = true;
    assert_eq!(rig.call(&Probes::above(0, 0.0), &inventory, 0), UseCommand::Block);
    assert_eq!(rig.service.take_notice(), Some(EasyPlaceNotice::NoNeighbour));
    rig.service.end_hold();
    rig.against[DOWN] = true;
    assert_eq!(rig.call(&Probes::above(0, 1.0), &inventory, 0), UseCommand::Place(Spots::on_floor(0)));
}

#[test]
fn a_real_block_hit_before_the_ghost_is_vanilla() {
    let mut rig = Rig::new(Spots::stone());
    let inventory = Inventories::with(&[(0, STONE)]);
    let mut probe = Probes::above(0, 0.0);
    probe.hit = Some(UseHit { cell: [0, 3, 0], face: Face::Up, distance: 2.0, replaceable: false });
    assert_eq!(rig.call(&probe, &inventory, 0), UseCommand::Vanilla);
    assert_eq!(rig.service.tried_len(), 0);
}

#[test]
fn a_replaceable_hit_on_a_ghost_spot_is_a_ghost_target() {
    let mut rig = Rig::new(Spots::stone());
    let inventory = Inventories::with(&[(0, STONE)]);
    let mut probe = Probes::above(0, 0.0);
    probe.hit = Some(UseHit { cell: [0, 0, 0], face: Face::Up, distance: 5.0, replaceable: true });
    assert_eq!(rig.call(&probe, &inventory, 0), UseCommand::Place(Spots::on_floor(0)));
}

#[test]
fn out_of_reach_is_vanilla() {
    let mut rig = Rig::new(Spots::stone());
    let inventory = Inventories::with(&[(0, STONE)]);
    let probe = UseProbe { reach: 3.0, ..Probes::above(0, 0.0) };
    assert_eq!(rig.call(&probe, &inventory, 0), UseCommand::Vanilla);
}

#[test]
fn once_a_hold_reaches_a_ghost_spot_real_blocks_are_blocked() {
    let mut rig = Rig::new(Spots::stone());
    let inventory = Inventories::with(&[(0, STONE)]);
    assert_eq!(rig.call(&Probes::above(0, 0.0), &inventory, 0), UseCommand::Place(Spots::on_floor(0)));
    let mut away = Probes::looking_up(0.2);
    away.hit = Some(UseHit { cell: [0, 9, 0], face: Face::Down, distance: 3.0, replaceable: false });
    assert_eq!(rig.call(&away, &inventory, 0), UseCommand::Block);
    assert_eq!(rig.service.telemetry().blocked_vanilla, 1);
    rig.service.end_hold();
    assert_eq!(rig.call(&away, &inventory, 0), UseCommand::Vanilla);
}

#[test]
fn a_hold_that_never_reaches_a_ghost_spot_stays_vanilla() {
    let mut rig = Rig::new(Spots::stone());
    let inventory = Inventories::with(&[(0, STONE)]);
    assert_eq!(rig.call(&Probes::looking_up(0.0), &inventory, 0), UseCommand::Vanilla);
    assert_eq!(rig.call(&Probes::looking_up(0.2), &inventory, 0), UseCommand::Vanilla);
    assert_eq!(rig.service.telemetry().blocked_vanilla, 0);
}

#[test]
fn a_real_block_in_front_with_no_ghost_behind_is_vanilla() {
    let mut rig = Rig::new(Spots::stone());
    let inventory = Inventories::with(&[(0, STONE)]);
    let mut probe = Probes::looking_up(0.0);
    probe.hit = Some(UseHit { cell: [0, 7, 0], face: Face::Down, distance: 2.0, replaceable: false });
    assert_eq!(rig.call(&probe, &inventory, 0), UseCommand::Vanilla);
    assert_eq!(rig.call(&probe, &inventory, 0), UseCommand::Vanilla);
}

#[test]
fn a_pending_spot_is_not_placed_twice() {
    let mut rig = Rig::new(Spots::stone());
    let inventory = Inventories::with(&[(0, STONE)]);
    assert_eq!(rig.call(&Probes::above(0, 0.0), &inventory, 0), UseCommand::Place(Spots::on_floor(0)));
    assert_eq!(rig.call(&Probes::above(0, 0.2), &inventory, 0), UseCommand::Block);
    assert_eq!(rig.service.telemetry().placed, 1);
    assert_eq!(rig.service.pending_len(), 1);
}

#[test]
fn a_rejected_spot_is_not_tried_again_in_the_hold() {
    let mut rig = Rig::new(Spots::stone());
    let inventory = Inventories::with(&[(0, STONE)]);
    assert_eq!(rig.call(&Probes::above(0, 0.0), &inventory, 0), UseCommand::Place(Spots::on_floor(0)));
    assert_eq!(rig.call(&Probes::above(0, 2.0), &inventory, 0), UseCommand::Block);
    assert_eq!(rig.service.telemetry().rejected, 1);
    assert_eq!(rig.service.pending_len(), 0);
    assert_eq!(rig.call(&Probes::above(0, 2.2), &inventory, 0), UseCommand::Block);
    assert_eq!(rig.service.telemetry().placed, 1);
}

#[test]
fn releasing_the_key_clears_the_tried_set() {
    let mut rig = Rig::new(Spots::stone());
    let inventory = Inventories::with(&[(0, STONE)]);
    assert_eq!(rig.call(&Probes::above(0, 0.0), &inventory, 0), UseCommand::Place(Spots::on_floor(0)));
    assert_eq!(rig.call(&Probes::above(0, 2.0), &inventory, 0), UseCommand::Block);
    rig.service.end_hold();
    assert_eq!(rig.service.tried_len(), 0);
    assert_eq!(rig.call(&Probes::above(0, 3.0), &inventory, 0), UseCommand::Place(Spots::on_floor(0)));
}

#[test]
fn a_double_slab_takes_two_placements() {
    let mut rig = Rig::new(Spots::single("oak_double_slab"));
    let inventory = Inventories::with(&[(0, b"minecraft:oak_slab")]);
    assert_eq!(rig.call(&Probes::above(0, 0.0), &inventory, 0), UseCommand::Place(Spots::on_floor(0)));
    rig.service.seen([0, 0, 0], b"minecraft:oak_slab", 0.1);
    assert_eq!(rig.call(&Probes::above(0, 2.0), &inventory, 0), UseCommand::Place(Spots::on_floor(0)));
    assert_eq!(rig.service.telemetry().confirmed, 1);
    assert_eq!(rig.service.telemetry().placed, 2);
    rig.service.seen([0, 0, 0], b"minecraft:oak_double_slab", 2.1);
    assert_eq!(rig.call(&Probes::above(0, 4.0), &inventory, 0), UseCommand::Block);
    let telemetry = rig.service.telemetry();
    assert_eq!((telemetry.confirmed, telemetry.wrong, telemetry.placed), (2, 0, 2));
}

#[test]
fn a_re_reported_single_slab_does_not_free_the_spot_again() {
    let mut rig = Rig::new(Spots::single("oak_double_slab"));
    let inventory = Inventories::with(&[(0, b"minecraft:oak_slab")]);
    assert_eq!(rig.call(&Probes::above(0, 0.0), &inventory, 0), UseCommand::Place(Spots::on_floor(0)));
    rig.service.seen([0, 0, 0], b"minecraft:oak_slab", 0.1);
    assert_eq!(rig.call(&Probes::above(0, 2.0), &inventory, 0), UseCommand::Place(Spots::on_floor(0)));
    rig.service.seen([0, 0, 0], b"minecraft:oak_slab", 2.1);
    assert_eq!(rig.call(&Probes::above(0, 4.0), &inventory, 0), UseCommand::Block);
    assert_eq!(rig.call(&Probes::above(0, 4.2), &inventory, 0), UseCommand::Block);
    let telemetry = rig.service.telemetry();
    assert_eq!((telemetry.confirmed, telemetry.placed), (2, 2));
}

#[test]
fn the_pending_set_holds_at_most_its_capacity() {
    let config = Rig::config(|config| config.pending_capacity = 4);
    let mut rig = Rig::with_config(Spots::row(5), config);
    let inventory = Inventories::with(&[(0, STONE)]);
    for x in 0..5 {
        let command = rig.call(&Probes::above(x, 0.0), &inventory, 0);
        assert_eq!(command, UseCommand::Place(Spots::on_floor(x)));
    }
    assert_eq!(rig.service.pending_len(), 4);
    assert_eq!(rig.service.telemetry().rejected, 1);
    assert_eq!(rig.service.telemetry().placed, 5);
    // The oldest entry (x 0) was evicted, so its block is not counted when the rest expire.
    rig.service.seen([0, 0, 0], STONE, 0.1);
    rig.call(&Probes::looking_up(2.0), &inventory, 0);
    let telemetry = rig.service.telemetry();
    assert_eq!((telemetry.confirmed, telemetry.rejected), (0, 5));
    assert_eq!(rig.service.pending_len(), 0);
}

#[test]
fn the_tried_set_holds_at_most_its_capacity() {
    let config = Rig::config(|config| {
        config.tried_capacity = 2;
        config.pending_timeout_ms = 100;
    });
    let mut rig = Rig::with_config(Spots::row(3), config);
    let inventory = Inventories::with(&[(0, STONE)]);
    for x in 0..3 {
        assert_eq!(rig.call(&Probes::above(x, 0.0), &inventory, 0), UseCommand::Place(Spots::on_floor(x)));
    }
    assert_eq!(rig.service.tried_len(), 2);
    assert_eq!(rig.call(&Probes::above(2, 1.0), &inventory, 0), UseCommand::Block);
    assert_eq!(rig.call(&Probes::above(0, 1.1), &inventory, 0), UseCommand::Place(Spots::on_floor(0)));
    assert_eq!(rig.service.tried_len(), 2);
}

#[test]
fn seen_blocks_count_confirmed_wrong_and_rejected() {
    let mut rig = Rig::new(Spots::row(4));
    let inventory = Inventories::with(&[(0, STONE)]);
    for x in 0..4 {
        rig.call(&Probes::above(x, 0.0), &inventory, 0);
    }
    rig.service.seen([0, 0, 0], STONE, 0.1);
    rig.service.seen([1, 0, 0], DIRT, 0.1);
    rig.service.seen([2, 0, 0], b"minecraft:air", 0.1);
    rig.call(&Probes::looking_up(2.0), &inventory, 0);
    let telemetry = rig.service.telemetry();
    assert_eq!((telemetry.confirmed, telemetry.wrong, telemetry.rejected), (1, 1, 2));
    assert_eq!(rig.service.pending_len(), 0);
}

#[test]
fn adventure_mode_is_vanilla() {
    let mut rig = Rig::new(Spots::stone());
    let inventory = Inventories::with(&[(0, STONE)]);
    let probe = UseProbe { game_mode: GameMode::Other, ..Probes::above(0, 0.0) };
    assert_eq!(rig.call(&probe, &inventory, 0), UseCommand::Vanilla);
}

#[test]
fn no_build_is_vanilla() {
    let mut rig = Rig::new(Spots::stone());
    rig.build = None;
    let inventory = Inventories::with(&[(0, STONE)]);
    assert_eq!(rig.call(&Probes::above(0, 0.0), &inventory, 0), UseCommand::Vanilla);
}

struct Halves;

impl Halves {
    // A one-cell build of a slab in the given half.
    fn slab(half: &str) -> GhostBuild {
        let state = [("minecraft:vertical_half", StateValue::Text(half.to_string()))];
        let structure = GhostFixture::stateful(BlockFactory::block("oak_slab", &state));
        let placed = [PlacedStructure { structure: Arc::clone(&structure), origin: [0, 0, 0] }];
        GhostAssembler::build(&placed, &[structure], [0, 0, 0], Rotation::R0, 1, None).unwrap()
    }

    fn place(rig: &mut Rig) -> UseCommand {
        let item = rig.item();
        let inventory = Inventories::with(&[(0, item.as_bytes())]);
        rig.call(&Probes::above(0, 0.0), &inventory, 0)
    }
}

#[test]
fn a_top_half_never_places_on_the_block_below() {
    let mut rig = Rig::new(Halves::slab("top"));
    assert_eq!(Halves::place(&mut rig), UseCommand::Block);
    assert_eq!(rig.service.take_notice(), Some(EasyPlaceNotice::NoNeighbour));
    rig.service.end_hold();
    rig.against[EAST] = true;
    let UseCommand::Place(placement) = Halves::place(&mut rig) else { panic!("not placed") };
    assert_eq!(placement.face, Face::West);
}

#[test]
fn a_bottom_half_never_places_under_the_block_above() {
    let mut rig = Rig::new(Halves::slab("bottom"));
    rig.against = [false; 6];
    rig.against[UP] = true;
    assert_eq!(Halves::place(&mut rig), UseCommand::Block);
    assert_eq!(rig.service.take_notice(), Some(EasyPlaceNotice::NoNeighbour));
    rig.service.end_hold();
    rig.against[DOWN] = true;
    let UseCommand::Place(placement) = Halves::place(&mut rig) else { panic!("not placed") };
    assert_eq!(placement.face, Face::Up);
}

#[test]
fn a_top_half_clicks_the_upper_part_of_a_side_face() {
    let mut rig = Rig::new(Halves::slab("top"));
    rig.against = [false; 6];
    rig.against[EAST] = true;
    let UseCommand::Place(placement) = Halves::place(&mut rig) else { panic!("not placed") };
    assert_eq!(placement.click, [0.0, 0.75, 0.5]);
}

#[test]
fn a_bottom_half_clicks_the_lower_part_of_a_side_face() {
    let mut rig = Rig::new(Halves::slab("bottom"));
    rig.against = [false; 6];
    rig.against[EAST] = true;
    let UseCommand::Place(placement) = Halves::place(&mut rig) else { panic!("not placed") };
    assert_eq!(placement.click, [0.0, 0.25, 0.5]);
}

#[test]
fn a_block_without_a_half_clicks_the_face_centre() {
    let mut rig = Rig::new(Spots::stone());
    rig.against = [true; 6];
    let UseCommand::Place(placement) = Halves::place(&mut rig) else { panic!("not placed") };
    let mut expected = [0.5; 3];
    match placement.face {
        Face::Down => expected[1] = 0.0,
        Face::Up => expected[1] = 1.0,
        Face::North => expected[2] = 0.0,
        Face::South => expected[2] = 1.0,
        Face::West => expected[0] = 0.0,
        Face::East => expected[0] = 1.0,
    }
    assert_eq!(placement.click, expected);
}

struct Ulps;

impl Ulps {
    // The value, one step below it and one step above it.
    fn around(value: f64) -> [f64; 3] {
        [value, f64::from_bits(value.to_bits() - 1), f64::from_bits(value.to_bits() + 1)]
    }
}

#[test]
fn a_filled_spot_is_vanilla_at_any_tie_of_the_hit_distance() {
    // From y 5.5 straight down the ray enters the spot through its top face at distance 4.5.
    for distance in Ulps::around(4.5) {
        let mut rig = Rig::new(Spots::stone());
        let inventory = Inventories::with(&[(0, STONE)]);
        let mut probe = Probes::above(0, 0.0);
        probe.hit = Some(UseHit { cell: [0, 0, 0], face: Face::Up, distance, replaceable: false });
        assert_eq!(rig.call(&probe, &inventory, 0), UseCommand::Vanilla, "distance {distance}");
        assert_eq!(rig.service.telemetry().placed, 0);
        assert_eq!(rig.service.tried_len(), 0);
    }
}

#[test]
fn a_spot_in_front_of_the_hit_block_is_a_target() {
    let mut rig = Rig::new(Spots::stone());
    let inventory = Inventories::with(&[(0, STONE)]);
    let mut probe = Probes::above(0, 0.0);
    probe.hit = Some(UseHit { cell: [0, -1, 0], face: Face::Up, distance: 5.5, replaceable: false });
    assert_eq!(rig.call(&probe, &inventory, 0), UseCommand::Place(Spots::on_floor(0)));
}

#[test]
fn the_second_half_of_a_double_slab_is_a_target_when_its_single_slab_is_hit() {
    for distance in Ulps::around(4.5) {
        let mut rig = Rig::new(Spots::single("oak_double_slab"));
        let inventory = Inventories::with(&[(0, b"minecraft:oak_slab")]);
        assert_eq!(rig.call(&Probes::above(0, 0.0), &inventory, 0), UseCommand::Place(Spots::on_floor(0)));
        rig.service.seen([0, 0, 0], b"minecraft:oak_slab", 0.1);
        let mut probe = Probes::above(0, 2.0);
        probe.hit = Some(UseHit { cell: [0, 0, 0], face: Face::Up, distance, replaceable: false });
        let command = rig.call(&probe, &inventory, 0);
        assert_eq!(command, UseCommand::Place(Spots::on_floor(0)), "distance {distance}");
    }
}

#[test]
fn a_filled_single_slab_spot_is_vanilla() {
    let mut rig = Rig::new(Spots::single("oak_double_slab"));
    let inventory = Inventories::with(&[(0, b"minecraft:oak_slab")]);
    let mut probe = Probes::above(0, 0.0);
    probe.hit = Some(UseHit { cell: [0, 0, 0], face: Face::Up, distance: 4.5, replaceable: false });
    assert_eq!(rig.call(&probe, &inventory, 0), UseCommand::Vanilla);
}

#[test]
fn the_cells_of_the_player_body_are_not_targets() {
    let mut rig = Rig::new(GhostFixture::blocks(&[([0, 0, 0], [0, 2, 0])]));
    let inventory = Inventories::with(&[(0, STONE)]);
    let mut probe = Probes::above(0, 0.0);
    probe.eye = [0.5, 2.6, 0.5];
    probe.body = Some(([0, 1, 0], [0, 2, 0]));
    assert_eq!(rig.call(&probe, &inventory, 0), UseCommand::Place(Spots::on_floor(0)));
}

#[test]
fn a_placement_not_sent_counts_as_not_sent_only() {
    let mut rig = Rig::new(Spots::row(2));
    let inventory = Inventories::with(&[(0, STONE)]);
    assert_eq!(rig.call(&Probes::above(0, 0.0), &inventory, 0), UseCommand::Place(Spots::on_floor(0)));
    assert_eq!(rig.call(&Probes::above(1, 0.1), &inventory, 0), UseCommand::Place(Spots::on_floor(1)));
    rig.service.not_sent([0, 0, 0]);
    rig.service.not_sent([5, 5, 5]);
    assert_eq!(rig.service.pending_len(), 1);
    rig.service.expire(2.0);
    let telemetry = rig.service.telemetry();
    assert_eq!((telemetry.placed, telemetry.not_sent, telemetry.rejected), (1, 1, 1));
}

#[test]
fn a_confirmed_block_broken_later_stays_confirmed() {
    let mut rig = Rig::new(Spots::stone());
    let inventory = Inventories::with(&[(0, STONE)]);
    assert_eq!(rig.call(&Probes::above(0, 0.0), &inventory, 0), UseCommand::Place(Spots::on_floor(0)));
    rig.service.seen([0, 0, 0], STONE, 0.1);
    rig.service.seen([0, 0, 0], b"minecraft:air", 0.2);
    rig.service.expire(2.0);
    let telemetry = rig.service.telemetry();
    assert_eq!((telemetry.confirmed, telemetry.rejected), (1, 0));
}

#[test]
fn a_first_half_seen_as_a_double_slab_needs_no_second_half() {
    let mut rig = Rig::new(Spots::single("oak_double_slab"));
    let inventory = Inventories::with(&[(0, b"minecraft:oak_slab")]);
    assert_eq!(rig.call(&Probes::above(0, 0.0), &inventory, 0), UseCommand::Place(Spots::on_floor(0)));
    rig.service.seen([0, 0, 0], b"minecraft:oak_slab", 0.1);
    rig.service.seen([0, 0, 0], b"minecraft:oak_double_slab", 0.2);
    rig.service.seen([0, 0, 0], b"minecraft:air", 0.3);
    rig.service.end_hold();
    let mut probe = Probes::above(0, 2.0);
    probe.hit = Some(UseHit { cell: [0, 0, 0], face: Face::Up, distance: 4.5, replaceable: false });
    assert_eq!(rig.call(&probe, &inventory, 0), UseCommand::Vanilla);
    assert_eq!(rig.service.telemetry().confirmed, 1);
}
