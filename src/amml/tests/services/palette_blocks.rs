use std::sync::Arc;

use alaydriem_stratum::services::PaletteBlocks;
use alaydriem_stratum::types::PaletteLookup;
use amml_api::BlockHandle;
use amml_api::testing::{FakeClient, FakeGame};
use stratum_core::structs::ghost::GhostPalette;
use stratum_core::structs::nbt::NbtTag;
use stratum_core::structs::structure::{BlockState, BlockStates, StateValue};

use crate::support::StratumRig;

const LEVER: u64 = 30;

fn nbt(state: BlockState) -> Vec<u8> {
    let palette = Arc::new(GhostPalette { entries: vec![state] });
    let mut out = Vec::new();
    palette.entry_nbt(0, &mut out).unwrap();
    out
}

fn lever() -> BlockState {
    BlockState::new(
        "minecraft:lever".to_string(),
        BlockStates::new(vec![
            ("open_bit".to_string(), StateValue::Bool(true)),
            ("lever_direction".to_string(), StateValue::Text("up_north_south".to_string())),
        ]),
    )
}

#[test]
fn the_entry_bytes_give_the_name_and_every_state_with_its_tag() {
    let (name, states) = PaletteBlocks::read(&nbt(lever())).unwrap();

    assert_eq!(name, "minecraft:lever");
    assert!(states.contains(&("open_bit".to_string(), NbtTag::Byte(1))));
    assert!(states.contains(&("lever_direction".to_string(), NbtTag::String("up_north_south".to_string()))));
}

#[test]
fn bytes_that_do_not_read_or_hold_no_name_give_nothing() {
    assert!(PaletteBlocks::read(&[]).is_none());
    assert!(PaletteBlocks::read(&[10, 0]).is_none());
}

#[test]
fn a_byte_state_is_asked_for_as_a_bool_and_the_match_is_exact() {
    let game = FakeGame::new();
    for area in StratumRig::areas() {
        game.offer_area(area);
    }
    game.open_gate();
    game.set_epoch(1);
    let mut world = StratumRig::world();
    world.states.insert(
        (
            "minecraft:lever".to_string(),
            vec![
                ("lever_direction".to_string(), "up_north_south".to_string()),
                ("open_bit".to_string(), "true".to_string()),
            ],
        ),
        BlockHandle::for_test(LEVER),
    );
    game.set_client(FakeClient { local_player: true, world_active: true, world: Some(world), ..FakeClient::default() });

    let found = PaletteBlocks::lookup(&game.ctx().blocks(), &nbt(lever()));

    assert_eq!(found, PaletteLookup::Exact(BlockHandle::for_test(LEVER)));
    assert_eq!(PaletteBlocks::lookup(&game.ctx().blocks(), &[]), PaletteLookup::Unknown);
}
