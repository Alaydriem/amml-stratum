use crate::services::ItemRules;
use crate::structs::build::BlockClass;
use crate::structs::structure::{BlockState, StateValue};

// Sorts block names into the classes that set build order.
pub struct BlockClassifier;

impl BlockClassifier {
    const FLUIDS: [&'static str; 4] = ["water", "flowing_water", "lava", "flowing_lava"];

    const GRAVITY: [&'static str; 11] = [
        "sand",
        "red_sand",
        "gravel",
        "anvil",
        "chipped_anvil",
        "damaged_anvil",
        "dragon_egg",
        "scaffolding",
        "pointed_dripstone",
        "suspicious_sand",
        "suspicious_gravel",
    ];

    // Blocks that always hang from the block above.
    const ALWAYS_HANGING: [&'static str; 3] = ["spore_blossom", "hanging_roots", "weeping_vines"];

    const GRAVITY_SUFFIXES: [&'static str; 1] = ["_concrete_powder"];

    const ATTACHED: [&'static str; 37] = [
        "torch",
        "ladder",
        "lever",
        "vine",
        "tripwire_hook",
        "tripwire",
        "frame",
        "glow_frame",
        "cocoa",
        "skull",
        "rail",
        "golden_rail",
        "detector_rail",
        "activator_rail",
        "redstone_wire",
        "dandelion",
        "poppy",
        "blue_orchid",
        "allium",
        "azure_bluet",
        "red_tulip",
        "orange_tulip",
        "white_tulip",
        "pink_tulip",
        "oxeye_daisy",
        "cornflower",
        "lily_of_the_valley",
        "wither_rose",
        "torchflower",
        "sunflower",
        "lilac",
        "rose_bush",
        "peony",
        "wall_sign",
        "wall_banner",
        "hanging_sign",
        "wall_hanging_sign",
    ];

    const ATTACHED_SUFFIXES: [&'static str; 8] =
        ["_torch", "_button", "_wall_sign", "_hanging_sign", "_wall_banner", "_carpet", "_pressure_plate", "_sapling"];

    // The class of a block by its name, with or without the `minecraft:` prefix.
    pub fn class(name: &str) -> BlockClass {
        let short = name.strip_prefix("minecraft:").unwrap_or(name);
        if Self::FLUIDS.contains(&short) {
            BlockClass::Fluid
        } else if Self::matches(short, &Self::GRAVITY, &Self::GRAVITY_SUFFIXES) {
            BlockClass::Gravity
        } else if Self::matches(short, &Self::ATTACHED, &Self::ATTACHED_SUFFIXES) {
            BlockClass::Attached
        } else {
            BlockClass::Plain
        }
    }

    // The class of a block by its name and states: a block that hangs from the block above is `Hanging`, which the
    // name alone does not tell for a lantern, a button, a lever, a hanging sign or a pointed dripstone.
    pub fn class_of(block: &BlockState) -> BlockClass {
        if Self::hangs(block) { BlockClass::Hanging } else { Self::class(&block.name) }
    }

    // The second block of a two-block pair: the upper half of a door or a tall plant, or the head of a bed. The game
    // places it with the first block, so a /setblock of it would fail.
    pub fn is_second_half(block: &BlockState) -> bool {
        let short = block.name.strip_prefix("minecraft:").unwrap_or(&block.name);
        let key = if short.ends_with("_door") || ItemRules::TALL_PLANTS.binary_search(&short).is_ok() {
            "upper_block_bit"
        } else if short == "bed" || short.ends_with("_bed") {
            "head_piece_bit"
        } else {
            return false;
        };
        match block.states.get(key) {
            Some(StateValue::Bool(on)) => *on,
            Some(StateValue::Int(value)) => *value != 0,
            _ => false,
        }
    }

    fn hangs(block: &BlockState) -> bool {
        let short = block.name.strip_prefix("minecraft:").unwrap_or(&block.name);
        let hanging_state = || match block.states.get("hanging") {
            Some(StateValue::Bool(on)) => Some(*on),
            Some(StateValue::Int(value)) => Some(*value != 0),
            _ => None,
        };
        if Self::ALWAYS_HANGING.contains(&short) || short.starts_with("cave_vines") {
            return true;
        }
        if short.contains("wall_hanging_sign") {
            return false;
        }
        if short == "hanging_sign" || short.ends_with("_hanging_sign") {
            return hanging_state().unwrap_or(true);
        }
        if matches!(short, "lantern" | "soul_lantern" | "pointed_dripstone") {
            return hanging_state().unwrap_or(false);
        }
        if short == "button" || short.ends_with("_button") {
            return matches!(block.states.get("facing_direction"), Some(StateValue::Int(0)));
        }
        if short == "lever" {
            return matches!(block.states.get("lever_direction"), Some(StateValue::Text(text)) if text.starts_with("down"));
        }
        false
    }

    fn matches(short: &str, names: &[&str], suffixes: &[&str]) -> bool {
        names.contains(&short) || suffixes.iter().any(|suffix| short.ends_with(suffix))
    }
}
