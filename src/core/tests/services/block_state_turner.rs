use stratum_core::services::BlockStateTurner;
use stratum_core::structs::marker::Rotation;
use stratum_core::structs::structure::{BlockState, StateValue};

use crate::support::block_factory::BlockFactory;

struct Rows;

impl Rows {
    fn text(value: &str) -> StateValue {
        StateValue::Text(value.to_string())
    }

    // One row per state: (state, value, R90 result).
    fn every_direction_state() -> Vec<(&'static str, StateValue, StateValue)> {
        vec![
            ("minecraft:cardinal_direction", Self::text("north"), Self::text("east")),
            ("minecraft:facing_direction", Self::text("west"), Self::text("north")),
            ("minecraft:block_face", Self::text("up"), Self::text("up")),
            // 0 down 1 up 2 north 3 south 4 west 5 east
            ("facing_direction", StateValue::Int(2), StateValue::Int(5)),
            // 0 south 1 west 2 north 3 east
            ("direction", StateValue::Int(0), StateValue::Int(1)),
            // 0 west 1 east 2 north 3 south
            ("coral_direction", StateValue::Int(2), StateValue::Int(1)),
            ("ground_sign_direction", StateValue::Int(0), StateValue::Int(4)),
            ("pillar_axis", Self::text("x"), Self::text("z")),
            ("portal_axis", Self::text("z"), Self::text("x")),
            ("torch_facing_direction", Self::text("north"), Self::text("east")),
            ("lever_direction", Self::text("up_north_south"), Self::text("up_east_west")),
            // ascending east -> ascending south
            ("rail_direction", StateValue::Int(2), StateValue::Int(5)),
            // curve south-east -> south-west
            ("rail_direction", StateValue::Int(6), StateValue::Int(7)),
            // bit 1 south -> bit 2 west
            ("vine_direction_bits", StateValue::Int(1), StateValue::Int(2)),
            // 0x10 north -> 0x20 east
            ("multi_face_direction_bits", StateValue::Int(0x10 | 0x1), StateValue::Int(0x20 | 0x1)),
            ("multi_face_direction_bits", StateValue::Int(0x20 | 0x2), StateValue::Int(0x4 | 0x2)),
            ("orientation", Self::text("north_up"), Self::text("east_up")),
        ]
    }
}

#[test]
fn stairs_turn_clockwise_by_weirdo_direction() {
    // weirdo_direction: 0 east, 1 west, 2 south, 3 north
    let stair = BlockFactory::block("oak_stairs", &[("weirdo_direction", StateValue::Int(0))]);
    let turned = BlockStateTurner::turn(&stair, Rotation::R90);
    assert_eq!(turned.states.get("weirdo_direction"), Some(&StateValue::Int(2)));
    assert_eq!(
        BlockStateTurner::turn(&stair, Rotation::R270).states.get("weirdo_direction"),
        Some(&StateValue::Int(3))
    );
}

#[test]
fn every_direction_state_turns() {
    for (state, value, expected) in Rows::every_direction_state() {
        let block = BlockFactory::block("test", &[(state, value)]);
        assert_eq!(BlockStateTurner::turn(&block, Rotation::R90).states.get(state), Some(&expected), "{state}");
    }
}

#[test]
fn wall_connections_move_to_the_turned_side() {
    let wall = BlockFactory::block(
        "cobblestone_wall",
        &[
            ("wall_connection_type_north", Rows::text("tall")),
            ("wall_connection_type_east", Rows::text("none")),
            ("wall_connection_type_south", Rows::text("short")),
            ("wall_connection_type_west", Rows::text("none")),
        ],
    );
    let turned = BlockStateTurner::turn(&wall, Rotation::R90);
    assert_eq!(turned.states.get("wall_connection_type_east"), Some(&Rows::text("tall")));
    assert_eq!(turned.states.get("wall_connection_type_west"), Some(&Rows::text("short")));
    assert_eq!(turned.states.get("wall_connection_type_north"), Some(&Rows::text("none")));
}

#[test]
fn four_quarter_turns_give_the_block_back() {
    for (state, value, _) in Rows::every_direction_state() {
        let original = BlockFactory::block("test", &[(state, value)]);
        let mut block: BlockState = original.clone();
        for _ in 0..4 {
            block = BlockStateTurner::turn(&block, Rotation::R90);
        }
        assert_eq!(block, original, "{state}");
    }
}

#[test]
fn r0_and_states_without_direction_are_unchanged() {
    let block = BlockFactory::block(
        "oak_stairs",
        &[
            ("upside_down_bit", StateValue::Bool(true)),
            ("minecraft:vertical_half", Rows::text("top")),
            ("color", Rows::text("red")),
        ],
    );
    assert_eq!(BlockStateTurner::turn(&block, Rotation::R90), block);
    assert_eq!(BlockStateTurner::turn(&block, Rotation::R270), block);

    for (state, value, _) in Rows::every_direction_state() {
        let directed = BlockFactory::block("test", &[(state, value)]);
        assert_eq!(BlockStateTurner::turn(&directed, Rotation::R0), directed, "{state}");
    }
}

#[test]
fn turned_block_keeps_name_and_state_order() {
    let block = BlockFactory::block(
        "oak_stairs",
        &[("upside_down_bit", StateValue::Bool(false)), ("weirdo_direction", StateValue::Int(1))],
    );
    let turned = BlockStateTurner::turn(&block, Rotation::R180);
    assert_eq!(turned.name, "minecraft:oak_stairs");
    let keys: Vec<&str> = turned.states.iter().map(|(key, _)| key).collect();
    assert_eq!(keys, ["upside_down_bit", "weirdo_direction"]);
    // west -> east
    assert_eq!(turned.states.get("weirdo_direction"), Some(&StateValue::Int(0)));
}

#[test]
fn trapdoors_turn_direction_in_the_stair_order() {
    // trapdoor direction: 0 east, 1 west, 2 south, 3 north
    for name in ["trapdoor", "iron_trapdoor"] {
        let door = BlockFactory::block(name, &[("direction", StateValue::Int(0))]);
        let turned = BlockStateTurner::turn(&door, Rotation::R90);
        assert_eq!(turned.states.get("direction"), Some(&StateValue::Int(2)), "{name}");
    }
}
