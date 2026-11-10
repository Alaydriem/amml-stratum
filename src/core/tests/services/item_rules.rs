use stratum_core::services::ItemRules;
use stratum_core::structs::material::{ItemId, ItemYield};
use stratum_core::structs::structure::StateValue;

use crate::support::BlockFactory;

#[test]
fn not_holdable_blocks_yield_nothing() {
    let rules = ItemRules::new();
    let names = [
        "air",
        "structure_void",
        "fire",
        "soul_fire",
        "portal",
        "end_portal",
        "end_gateway",
        "piston_arm_collision",
        "sticky_piston_arm_collision",
        "moving_block",
        "bubble_column",
        "frosted_ice",
        "light_block_0",
        "light_block_15",
        "flowing_water",
        "flowing_lava",
    ];
    for name in names {
        assert_eq!(rules.items_for(&BlockFactory::plain(name)), ItemYield::None, "{name}");
    }
}

#[test]
fn upper_halves_yield_nothing() {
    let rules = ItemRules::new();
    let upper = [("upper_block_bit", StateValue::Bool(true))];
    let lower = [("upper_block_bit", StateValue::Bool(false))];
    let doors = ["oak_door", "iron_door", "copper_door", "pale_oak_door"];
    let plants = ["sunflower", "lilac", "rose_bush", "peony", "tall_grass", "large_fern", "pitcher_plant"];
    for name in doors.iter().chain(plants.iter()) {
        assert_eq!(rules.items_for(&BlockFactory::block(name, &upper)), ItemYield::None, "{name} upper");
        assert_eq!(rules.items_for(&BlockFactory::block(name, &lower)), BlockFactory::one(name, 1), "{name} lower");
    }
    let head = [("head_piece_bit", StateValue::Bool(true))];
    let foot = [("head_piece_bit", StateValue::Bool(false))];
    assert_eq!(rules.items_for(&BlockFactory::block("bed", &head)), ItemYield::None);
    assert_eq!(rules.items_for(&BlockFactory::block("bed", &foot)), BlockFactory::one("bed", 1));
    assert_eq!(rules.items_for(&BlockFactory::block("red_bed", &head)), ItemYield::None);
    assert_eq!(rules.items_for(&BlockFactory::block("red_bed", &foot)), BlockFactory::one("red_bed", 1));
}

#[test]
fn state_bits_stored_as_integers_still_count() {
    let rules = ItemRules::new();
    let upper = [("upper_block_bit", StateValue::Int(1))];
    assert_eq!(rules.items_for(&BlockFactory::block("oak_door", &upper)), ItemYield::None);
}

#[test]
fn double_slab_yields_two_slabs() {
    let rules = ItemRules::new();
    assert_eq!(rules.items_for(&BlockFactory::plain("oak_double_slab")), BlockFactory::one("oak_slab", 2));
    assert_eq!(
        rules.items_for(&BlockFactory::plain("stone_brick_double_slab")),
        BlockFactory::one("stone_brick_slab", 2)
    );
    assert_eq!(rules.items_for(&BlockFactory::plain("oak_slab")), BlockFactory::one("oak_slab", 1));
}

#[test]
fn wall_and_standing_variants_yield_their_item() {
    let rules = ItemRules::new();
    let cases = [
        ("wall_sign", "oak_sign"),
        ("spruce_wall_sign", "spruce_sign"),
        ("standing_sign", "oak_sign"),
        ("spruce_standing_sign", "spruce_sign"),
        ("wall_banner", "banner"),
        ("standing_banner", "banner"),
    ];
    for (block, item) in cases {
        assert_eq!(rules.items_for(&BlockFactory::plain(block)), BlockFactory::one(item, 1), "{block}");
    }
    assert_eq!(rules.stack_size(&BlockFactory::item("spruce_sign")), 16);
}

#[test]
fn only_liquid_sources_yield_buckets() {
    let rules = ItemRules::new();
    let source = [("liquid_depth", StateValue::Int(0))];
    let flowing = [("liquid_depth", StateValue::Int(3))];
    assert_eq!(rules.items_for(&BlockFactory::block("water", &source)), BlockFactory::one("water_bucket", 1));
    assert_eq!(rules.items_for(&BlockFactory::block("lava", &source)), BlockFactory::one("lava_bucket", 1));
    assert_eq!(rules.items_for(&BlockFactory::block("water", &flowing)), ItemYield::None);
    assert_eq!(rules.items_for(&BlockFactory::block("lava", &flowing)), ItemYield::None);
}

#[test]
fn changed_items_map_to_their_item() {
    let rules = ItemRules::new();
    let cases = [
        ("redstone_wire", "redstone"),
        ("unpowered_repeater", "repeater"),
        ("powered_repeater", "repeater"),
        ("unpowered_comparator", "comparator"),
        ("powered_comparator", "comparator"),
        ("lit_furnace", "furnace"),
        ("lit_blast_furnace", "blast_furnace"),
        ("lit_smoker", "smoker"),
        ("unlit_redstone_torch", "redstone_torch"),
        ("lit_redstone_lamp", "redstone_lamp"),
        ("lit_redstone_ore", "redstone_ore"),
        ("lit_deepslate_redstone_ore", "deepslate_redstone_ore"),
        ("wheat", "wheat_seeds"),
        ("carrots", "carrot"),
        ("potatoes", "potato"),
        ("beetroot", "beetroot_seeds"),
        ("pumpkin_stem", "pumpkin_seeds"),
        ("melon_stem", "melon_seeds"),
        ("cocoa", "cocoa_beans"),
        ("tripwire", "string"),
        ("farmland", "dirt"),
        ("grass_path", "dirt"),
    ];
    for (block, item) in cases {
        assert_eq!(rules.items_for(&BlockFactory::plain(block)), BlockFactory::one(item, 1), "{block}");
    }
}

#[test]
fn count_states_multiply() {
    let rules = ItemRules::new();
    let candles = [("candles", StateValue::Int(2))];
    assert_eq!(rules.items_for(&BlockFactory::block("candle", &candles)), BlockFactory::one("candle", 3));
    assert_eq!(rules.items_for(&BlockFactory::block("red_candle", &candles)), BlockFactory::one("red_candle", 3));
    // Values 0 and 1 reach the rules as booleans.
    let one_candle = [("candles", StateValue::Bool(false))];
    assert_eq!(rules.items_for(&BlockFactory::block("candle", &one_candle)), BlockFactory::one("candle", 1));
    let two_candles = [("candles", StateValue::Bool(true))];
    assert_eq!(rules.items_for(&BlockFactory::block("candle", &two_candles)), BlockFactory::one("candle", 2));

    let pickles = [("cluster_count", StateValue::Int(3))];
    assert_eq!(rules.items_for(&BlockFactory::block("sea_pickle", &pickles)), BlockFactory::one("sea_pickle", 4));

    let words = [("one_egg", 1), ("two_egg", 2), ("three_egg", 3), ("four_egg", 4)];
    for (word, count) in words {
        let eggs = [("turtle_egg_count", StateValue::Text(word.to_string()))];
        assert_eq!(rules.items_for(&BlockFactory::block("turtle_egg", &eggs)), BlockFactory::one("turtle_egg", count));
    }

    let snow = [("height", StateValue::Int(6))];
    assert_eq!(rules.items_for(&BlockFactory::block("snow_layer", &snow)), BlockFactory::one("snow_layer", 7));
}

#[test]
fn count_blocks_without_their_state_count_once() {
    let rules = ItemRules::new();
    for name in ["candle", "sea_pickle", "turtle_egg", "snow_layer"] {
        assert_eq!(rules.items_for(&BlockFactory::plain(name)), BlockFactory::one(name, 1), "{name}");
    }
}

#[test]
fn candle_cake_yields_cake_and_candle() {
    let rules = ItemRules::new();
    assert_eq!(rules.items_for(&BlockFactory::plain("candle_cake")), BlockFactory::two("cake", 1, "candle", 1));
    assert_eq!(
        rules.items_for(&BlockFactory::plain("blue_candle_cake")),
        BlockFactory::two("cake", 1, "blue_candle", 1)
    );
}

#[test]
fn other_blocks_yield_themselves() {
    let rules = ItemRules::new();
    assert_eq!(rules.items_for(&BlockFactory::plain("stone_bricks")), BlockFactory::one("stone_bricks", 1));
    assert_eq!(rules.items_for(&BlockFactory::plain("oak_door")), BlockFactory::one("oak_door", 1));
}

#[test]
fn other_namespaces_pass_through() {
    let rules = ItemRules::new();
    assert_eq!(
        rules.items_for(&BlockFactory::namespaced("mymod:air")),
        ItemYield::One(ItemId("mymod:air".to_string()), 1)
    );
}

#[test]
fn stack_sizes_follow_the_table() {
    let rules = ItemRules::new();
    let ones = ["water_bucket", "lava_bucket", "bed", "red_bed", "cake"];
    let sixteens =
        ["oak_sign", "spruce_hanging_sign", "banner", "armor_stand", "snowball", "egg", "ender_pearl", "honey_bottle"];
    for name in ones {
        assert_eq!(rules.stack_size(&BlockFactory::item(name)), 1, "{name}");
    }
    for name in sixteens {
        assert_eq!(rules.stack_size(&BlockFactory::item(name)), 16, "{name}");
    }
    assert_eq!(rules.stack_size(&BlockFactory::item("stone")), 64);
    assert_eq!(rules.stack_size(&BlockFactory::item("bedrock")), 64);
}

#[test]
fn wall_hanging_signs_yield_their_hanging_sign() {
    let rules = ItemRules::new();
    let cases =
        [("spruce_wall_hanging_sign", "spruce_hanging_sign"), ("pale_oak_wall_hanging_sign", "pale_oak_hanging_sign")];
    for (block, item) in cases {
        assert_eq!(rules.items_for(&BlockFactory::plain(block)), BlockFactory::one(item, 1), "{block}");
    }
    assert_eq!(
        rules.items_for(&BlockFactory::plain("spruce_hanging_sign")),
        BlockFactory::one("spruce_hanging_sign", 1)
    );
}

#[test]
fn lookup_tables_are_sorted() {
    assert!(ItemRules::NOT_HOLDABLE.is_sorted());
    assert!(ItemRules::TALL_PLANTS.is_sorted());
    assert!(ItemRules::RENAMED.is_sorted_by_key(|(from, _)| *from));
}
