use std::sync::Arc;

use alaydriem_stratum::ghost::BlockResolver;
use amml_api::BlockHandle;
use amml_api::testing::{FakeClient, FakeGame};
use stratum_core::structs::ghost::GhostPalette;
use stratum_core::structs::structure::{BlockState, BlockStates, StateValue};

use crate::support::StratumRig;

const STAIRS: u64 = 20;
const STAIRS_TURNED: u64 = 21;

fn game() -> FakeGame {
    let game = FakeGame::new();
    for area in StratumRig::areas() {
        game.offer_area(area);
    }
    game.open_gate();
    game.set_epoch(1);
    let mut world = StratumRig::world();
    world.states.insert(
        ("minecraft:oak_stairs".to_string(), vec![("weirdo_direction".to_string(), "0".to_string())]),
        BlockHandle::for_test(STAIRS),
    );
    world.default_states.insert("minecraft:oak_stairs".to_string(), BlockHandle::for_test(STAIRS));
    world.rotated.insert((BlockHandle::for_test(STAIRS), 1), BlockHandle::for_test(STAIRS_TURNED));
    game.set_client(FakeClient { local_player: true, world_active: true, world: Some(world), ..FakeClient::default() });
    game
}

fn entries(states: &[BlockState]) -> Vec<Vec<u8>> {
    let palette = Arc::new(GhostPalette { entries: states.to_vec() });
    (0..states.len() as u16)
        .map(|id| {
            let mut out = Vec::new();
            palette.entry_nbt(id, &mut out).unwrap();
            out
        })
        .collect()
}

fn stairs(direction: &str, value: i32) -> BlockState {
    BlockState::new(
        "minecraft:oak_stairs".to_string(),
        BlockStates::new(vec![(direction.to_string(), StateValue::Int(value))]),
    )
}

#[test]
fn entries_resolve_exact_then_default_then_unknown_and_the_fallbacks_are_counted() {
    let game = game();
    let blocks = game.ctx().blocks();
    let mut resolver = BlockResolver::new();
    assert!(resolver.join(&blocks));
    let states = [
        BlockState::new("minecraft:stone".to_string(), BlockStates::default()),
        stairs("weirdo_direction", 7),
        BlockState::new("minecraft:nothing".to_string(), BlockStates::default()),
    ];
    let bytes = entries(&states);

    resolver.resolve(&blocks, bytes.iter().map(Vec::as_slice), 0);

    assert_eq!(resolver.block(0), Some(BlockHandle::for_test(StratumRig::STONE)));
    assert_eq!(resolver.block(1), Some(BlockHandle::for_test(STAIRS)));
    assert_eq!(resolver.block(2), Some(BlockHandle::for_test(StratumRig::UNKNOWN)));
    assert_eq!(resolver.take_default_state(), 1);
    assert_eq!(resolver.take_unknown(), 1);
    assert_eq!(resolver.take_unknown(), 0, "a count is taken once");
}

#[test]
fn a_rotation_turns_each_block_with_the_game_but_never_the_unknown_block() {
    let game = game();
    let blocks = game.ctx().blocks();
    let mut resolver = BlockResolver::new();
    resolver.join(&blocks);
    let states =
        [stairs("weirdo_direction", 0), BlockState::new("minecraft:nothing".to_string(), BlockStates::default())];
    let bytes = entries(&states);

    resolver.resolve(&blocks, bytes.iter().map(Vec::as_slice), 1);

    assert_eq!(resolver.block(0), Some(BlockHandle::for_test(STAIRS_TURNED)));
    assert_eq!(game.rotations(), vec![(BlockHandle::for_test(STAIRS), 1)]);
}

#[test]
fn air_and_empty_cells_hold_no_block_but_every_id_names_a_handle_for_the_mesher() {
    let game = game();
    let blocks = game.ctx().blocks();
    let mut resolver = BlockResolver::new();
    resolver.join(&blocks);
    let states = [BlockState::new("minecraft:air".to_string(), BlockStates::default())];
    let bytes = entries(&states);

    resolver.resolve(&blocks, bytes.iter().map(Vec::as_slice), 0);

    assert_eq!(resolver.block(0), None);
    assert_eq!(resolver.block(BlockResolver::EMPTY), None);
    assert_eq!(resolver.handles(), &[BlockHandle::for_test(StratumRig::AIR)]);
}

#[test]
fn a_world_with_no_air_cannot_be_joined_and_resolves_nothing() {
    let game = FakeGame::new();
    for area in StratumRig::areas() {
        game.offer_area(area);
    }
    game.open_gate();
    game.set_epoch(1);
    game.set_client(FakeClient { local_player: true, world_active: true, ..FakeClient::default() });
    let blocks = game.ctx().blocks();
    let mut resolver = BlockResolver::new();

    assert!(!resolver.join(&blocks));
    let bytes = entries(&[BlockState::new("minecraft:stone".to_string(), BlockStates::default())]);
    resolver.resolve(&blocks, bytes.iter().map(Vec::as_slice), 0);
    assert!(!resolver.is_resolved());
}
