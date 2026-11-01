use crate::structs::material::{ItemId, ItemYield};
use crate::structs::structure::{BlockState, StateValue};

// Prefix of the light block family, which no player can hold.
const LIGHT_BLOCK_PREFIX: &str = "light_block_";

// Items that stack to 1. Items ending in `_bed` stack to 1 as well.
const STACK_ONE: &[&str] = &["bed", "cake", "lava_bucket", "water_bucket"];

// Items that stack to 16. Items ending in `_sign` stack to 16 as well.
const STACK_SIXTEEN: &[&str] = &["armor_stand", "banner", "egg", "ender_pearl", "honey_bottle", "snowball"];

const NAMESPACE: &str = "minecraft:";
const DEFAULT_STACK_SIZE: u32 = 64;

// Maps a placed block to the items a player must gather to place it.
#[derive(Debug, Clone, Default)]
pub struct ItemRules;

impl ItemRules {
    // The namespace of vanilla block and item names.
    pub const NAMESPACE: &str = NAMESPACE;

    // Blocks that no player can hold, sorted by name.
    pub const NOT_HOLDABLE: &[&str] = &[
        "air",
        "bubble_column",
        "end_gateway",
        "end_portal",
        "fire",
        "flowing_lava",
        "flowing_water",
        "frosted_ice",
        "moving_block",
        "piston_arm_collision",
        "portal",
        "soul_fire",
        "sticky_piston_arm_collision",
        "structure_void",
    ];

    // Blocks whose item has another name, sorted by block name.
    pub const RENAMED: &[(&str, &str)] = &[
        ("beetroot", "beetroot_seeds"),
        ("carrots", "carrot"),
        ("cocoa", "cocoa_beans"),
        ("farmland", "dirt"),
        ("grass_path", "dirt"),
        ("lit_blast_furnace", "blast_furnace"),
        ("lit_deepslate_redstone_ore", "deepslate_redstone_ore"),
        ("lit_furnace", "furnace"),
        ("lit_redstone_lamp", "redstone_lamp"),
        ("lit_redstone_ore", "redstone_ore"),
        ("lit_smoker", "smoker"),
        ("melon_stem", "melon_seeds"),
        ("potatoes", "potato"),
        ("powered_comparator", "comparator"),
        ("powered_repeater", "repeater"),
        ("pumpkin_stem", "pumpkin_seeds"),
        ("redstone_wire", "redstone"),
        ("standing_banner", "banner"),
        ("standing_sign", "oak_sign"),
        ("tripwire", "string"),
        ("unlit_redstone_torch", "redstone_torch"),
        ("unpowered_comparator", "comparator"),
        ("unpowered_repeater", "repeater"),
        ("wall_banner", "banner"),
        ("wall_sign", "oak_sign"),
        ("wheat", "wheat_seeds"),
    ];

    // Tall plants whose upper half is a second block, sorted by name.
    pub const TALL_PLANTS: &[&str] =
        &["large_fern", "lilac", "peony", "pitcher_plant", "rose_bush", "sunflower", "tall_grass"];

    pub fn new() -> Self {
        Self
    }

    pub fn items_for(&self, block: &BlockState) -> ItemYield {
        let Some(name) = block.name.strip_prefix(NAMESPACE) else {
            return ItemYield::One(ItemId(block.name.clone()), 1);
        };

        if Self::NOT_HOLDABLE.binary_search(&name).is_ok() || name.starts_with(LIGHT_BLOCK_PREFIX) {
            return ItemYield::None;
        }
        if Self::is_upper_half(name, block) {
            return ItemYield::None;
        }
        if let Some(base) = name.strip_suffix("_double_slab") {
            return ItemYield::One(Self::item(&format!("{base}_slab")), 2);
        }
        if name == "candle_cake" {
            return ItemYield::Two(Self::item("cake"), 1, Self::item("candle"), 1);
        }
        if let Some(base) = name.strip_suffix("_candle_cake") {
            return ItemYield::Two(Self::item("cake"), 1, Self::item(&format!("{base}_candle")), 1);
        }
        if name == "candle" || name.ends_with("_candle") {
            return ItemYield::One(Self::item(name), Self::count_state(block, "candles").saturating_add(1));
        }
        match name {
            "sea_pickle" => {
                return ItemYield::One(Self::item(name), Self::count_state(block, "cluster_count").saturating_add(1));
            }
            "snow_layer" => {
                return ItemYield::One(Self::item(name), Self::count_state(block, "height").saturating_add(1));
            }
            "turtle_egg" => return ItemYield::One(Self::item(name), Self::egg_count(block)),
            "water" => return Self::bucket(block, "water_bucket"),
            "lava" => return Self::bucket(block, "lava_bucket"),
            _ => {}
        }
        if let Ok(position) = Self::RENAMED.binary_search_by(|(from, _)| (*from).cmp(name)) {
            return ItemYield::One(Self::item(Self::RENAMED[position].1), 1);
        }
        if let Some(wood) = name.strip_suffix("_wall_hanging_sign") {
            return ItemYield::One(Self::item(&format!("{wood}_hanging_sign")), 1);
        }
        if let Some(wood) = name.strip_suffix("_wall_sign") {
            return ItemYield::One(Self::item(&format!("{wood}_sign")), 1);
        }
        if let Some(wood) = name.strip_suffix("_standing_sign") {
            return ItemYield::One(Self::item(&format!("{wood}_sign")), 1);
        }
        ItemYield::One(Self::item(name), 1)
    }

    pub fn stack_size(&self, item: &ItemId) -> u32 {
        let name = item.short_name();
        if STACK_ONE.contains(&name) || name.ends_with("_bed") {
            1
        } else if STACK_SIXTEEN.contains(&name) || name.ends_with("_sign") {
            16
        } else {
            DEFAULT_STACK_SIZE
        }
    }

    fn item(name: &str) -> ItemId {
        ItemId(format!("{NAMESPACE}{name}"))
    }

    // Doors and tall plants store the upper half as a second block; beds store the head.
    fn is_upper_half(name: &str, block: &BlockState) -> bool {
        if name.ends_with("_door") || Self::TALL_PLANTS.binary_search(&name).is_ok() {
            return Self::flag(block, "upper_block_bit");
        }
        if name == "bed" || name.ends_with("_bed") {
            return Self::flag(block, "head_piece_bit");
        }
        false
    }

    // Only a source block (depth 0) can be picked up with a bucket.
    fn bucket(block: &BlockState, bucket: &str) -> ItemYield {
        match Self::number(block, "liquid_depth") {
            None | Some(0) => ItemYield::One(Self::item(bucket), 1),
            Some(_) => ItemYield::None,
        }
    }

    // A state that counts items, zero when it is missing.
    fn count_state(block: &BlockState, key: &str) -> u32 {
        let value = Self::number(block, key).unwrap_or(0).clamp(0, i64::from(u32::MAX) - 1);
        u32::try_from(value).unwrap_or(0)
    }

    fn egg_count(block: &BlockState) -> u32 {
        match block.states.get("turtle_egg_count") {
            Some(StateValue::Text(text)) => match text.as_str() {
                "two_egg" => 2,
                "three_egg" => 3,
                "four_egg" => 4,
                _ => 1,
            },
            _ => 1,
        }
    }

    // A numeric state. The parser reads the byte values 0 and 1 as booleans.
    fn number(block: &BlockState, key: &str) -> Option<i64> {
        match block.states.get(key)? {
            StateValue::Int(value) => Some(i64::from(*value)),
            StateValue::Bool(value) => Some(i64::from(*value)),
            StateValue::Text(_) => None,
        }
    }

    fn flag(block: &BlockState, key: &str) -> bool {
        Self::number(block, key).is_some_and(|value| value != 0)
    }
}
