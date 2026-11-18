use crate::structs::marker::Rotation;
use crate::structs::structure::{BlockState, BlockStates, StateValue};

// Turns the direction states of a block by quarter turns, clockwise seen from above.
pub struct BlockStateTurner;

impl BlockStateTurner {
    // Compass sides in clockwise order.
    const COMPASS: [&'static str; 4] = ["north", "east", "south", "west"];

    // Integer codes in clockwise order, one cycle per encoding.
    // facing_direction: 0 down 1 up 2 north 3 south 4 west 5 east
    const FACING: [i32; 4] = [2, 5, 3, 4];
    // direction: 0 south 1 west 2 north 3 east
    const DIRECTION: [i32; 4] = [0, 1, 2, 3];
    // coral_direction: 0 west 1 east 2 north 3 south
    const CORAL: [i32; 4] = [2, 1, 3, 0];
    // weirdo_direction: 0 east 1 west 2 south 3 north
    const WEIRDO: [i32; 4] = [3, 0, 2, 1];
    // rail_direction ascending: 2 east 3 west 4 north 5 south
    const RAIL_ASCENDING: [i32; 4] = [4, 2, 5, 3];
    // rail_direction curves: 6 south-east 7 south-west 8 north-west 9 north-east
    const RAIL_CURVE: [i32; 4] = [6, 7, 8, 9];

    const WALL_PREFIX: &'static str = "wall_connection_type_";

    // The block with every direction state turned. Other states and the state order stay.
    pub fn turn(block: &BlockState, rotation: Rotation) -> BlockState {
        let quarters = usize::from(rotation.quarters());
        if quarters == 0 {
            return block.clone();
        }
        let pairs = block
            .states
            .iter()
            .map(|(key, value)| {
                let turned = Self::turn_wall(&block.states, key, quarters)
                    .unwrap_or_else(|| Self::turn_value(&block.name, key, value, quarters));
                (key.to_string(), turned)
            })
            .collect();
        BlockState::new(block.name.clone(), BlockStates::new(pairs))
    }

    fn turn_value(name: &str, key: &str, value: &StateValue, quarters: usize) -> StateValue {
        match value {
            StateValue::Int(code) => StateValue::Int(Self::turn_int(name, key, *code, quarters)),
            StateValue::Text(text) => StateValue::Text(Self::turn_text(key, text, quarters)),
            StateValue::Bool(_) => value.clone(),
        }
    }

    fn turn_int(name: &str, key: &str, code: i32, quarters: usize) -> i32 {
        match key {
            "direction" if name.ends_with("trapdoor") => Self::step(&Self::WEIRDO, code, quarters),
            "facing_direction" => Self::step(&Self::FACING, code, quarters),
            "direction" => Self::step(&Self::DIRECTION, code, quarters),
            "coral_direction" => Self::step(&Self::CORAL, code, quarters),
            "weirdo_direction" => Self::step(&Self::WEIRDO, code, quarters),
            "ground_sign_direction" => (code + 4 * quarters as i32).rem_euclid(16),
            "rail_direction" => Self::turn_rail(code, quarters),
            "vine_direction_bits" => Self::turn_vine(code, quarters),
            "multi_face_direction_bits" => Self::turn_multi_face(code, quarters),
            _ => code,
        }
    }

    fn turn_text(key: &str, text: &str, quarters: usize) -> String {
        match key {
            "minecraft:cardinal_direction"
            | "minecraft:facing_direction"
            | "minecraft:block_face"
            | "torch_facing_direction" => Self::turn_side(text, quarters),
            "pillar_axis" | "portal_axis" => Self::turn_axis(text, quarters),
            "lever_direction" => Self::turn_lever(text, quarters),
            "orientation" => Self::turn_orientation(text, quarters),
            _ => text.to_string(),
        }
    }

    // The entry `quarters` places along the cycle. A code outside the cycle stays.
    fn step(cycle: &[i32; 4], code: i32, quarters: usize) -> i32 {
        match cycle.iter().position(|entry| *entry == code) {
            Some(index) => cycle[(index + quarters) % 4],
            None => code,
        }
    }

    // A compass side turned. Any other word, such as up or down, stays.
    fn turn_side(side: &str, quarters: usize) -> String {
        match Self::COMPASS.iter().position(|entry| *entry == side) {
            Some(index) => Self::COMPASS[(index + quarters) % 4].to_string(),
            None => side.to_string(),
        }
    }

    // Axes x and z swap on an odd count of quarter turns.
    fn turn_axis(axis: &str, quarters: usize) -> String {
        match (axis, quarters % 2) {
            ("x", 1) => "z".to_string(),
            ("z", 1) => "x".to_string(),
            _ => axis.to_string(),
        }
    }

    // Lever values are a side, such as east, or a half and an axis pair, such as up_north_south.
    fn turn_lever(value: &str, quarters: usize) -> String {
        if quarters % 2 == 1 && value.ends_with("north_south") {
            return value.replace("north_south", "east_west");
        }
        if quarters % 2 == 1 && value.ends_with("east_west") {
            return value.replace("east_west", "north_south");
        }
        Self::turn_side(value, quarters)
    }

    // Orientation joins a front and a top, such as north_up. Each side word turns on its own.
    fn turn_orientation(value: &str, quarters: usize) -> String {
        value.split('_').map(|word| Self::turn_side(word, quarters)).collect::<Vec<_>>().join("_")
    }

    fn turn_rail(code: i32, quarters: usize) -> i32 {
        match code {
            0 | 1 => {
                if quarters % 2 == 1 {
                    1 - code
                } else {
                    code
                }
            }
            2..=5 => Self::step(&Self::RAIL_ASCENDING, code, quarters),
            6..=9 => Self::step(&Self::RAIL_CURVE, code, quarters),
            _ => code,
        }
    }

    // Vine bits: 1 south, 2 west, 4 north, 8 east. Clockwise moves each bit one place up and wraps.
    fn turn_vine(bits: i32, quarters: usize) -> i32 {
        let bits = bits & 0b1111;
        ((bits << quarters) | (bits >> (4 - quarters))) & 0b1111
    }

    // Multi face bits: 0x1 down, 0x2 up, 0x4 south, 0x8 west, 0x10 north, 0x20 east. Down and up stay.
    // The four side bits cycle clockwise from south to west, north, east and back.
    fn turn_multi_face(bits: i32, quarters: usize) -> i32 {
        let sides = (bits >> 2) & 0b1111;
        let turned = ((sides << quarters) | (sides >> (4 - quarters))) & 0b1111;
        (bits & !0b111100) | (turned << 2)
    }

    // A wall side takes the value of the side that turns onto it. None when the key is no wall side.
    fn turn_wall(states: &BlockStates, key: &str, quarters: usize) -> Option<StateValue> {
        let side = key.strip_prefix(Self::WALL_PREFIX)?;
        let source = Self::turn_side(side, (4 - quarters) % 4);
        states.get(&format!("{}{source}", Self::WALL_PREFIX)).cloned()
    }
}
