use alaydriem_stratum::services::GateRunner;
use amml_game::BlockPos;
use stratum_core::services::NbtReader;
use stratum_core::structs::nbt::{NbtLimits, NbtTag};

#[test]
fn the_gate_block_is_three_above_the_feet_at_negative_positions() {
    // a player whose eye is at y 65.62 stands on y 64; x and z round down, also below zero
    let pos = GateRunner::gate_block([-0.5, 65.62, -10.2]);

    assert_eq!(pos, BlockPos { x: -1, y: 67, z: -11 });
}

#[test]
fn a_feet_level_a_hair_below_a_whole_block_counts_as_that_block() {
    // seen in game: a player standing on y 72 has its eye at 73.6198, so its feet at 71.9998
    let pos = GateRunner::gate_block([4.28, 73.6198, 263.76]);

    assert_eq!(pos, BlockPos { x: 4, y: 75, z: 263 });
}

#[test]
fn a_feet_level_on_a_lower_block_top_stays_in_its_block() {
    // soul sand is 0.875 high: a player on the one at y 63 has its feet at 63.875
    let pos = GateRunner::gate_block([3.0, 63.875 + 1.62, 4.99]);

    assert_eq!(pos, BlockPos { x: 3, y: 66, z: 4 });
}

#[test]
fn the_entry_bytes_are_a_palette_entry_with_name_states_and_version() {
    let bytes = GateRunner::entry_nbt();

    let (_, root) = NbtReader::read(&bytes, NbtLimits::STRUCTURE).unwrap();

    assert_eq!(root.get("name"), Some(&NbtTag::String("minecraft:oak_stairs".to_string())));
    assert_eq!(root.get("version"), Some(&NbtTag::Int(18_168_865)));
    let Some(NbtTag::Compound(states)) = root.get("states") else { panic!("no states compound") };
    assert_eq!(states.get("weirdo_direction"), Some(&NbtTag::Int(2)));
}
