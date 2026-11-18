use stratum_core::services::BlockClassifier;
use stratum_core::structs::build::BlockClass;
use stratum_core::structs::structure::StateValue;

use crate::support::BlockFactory;

#[test]
fn classes() {
    assert_eq!(BlockClassifier::class("minecraft:water"), BlockClass::Fluid);
    assert_eq!(BlockClassifier::class("minecraft:flowing_lava"), BlockClass::Fluid);
    assert_eq!(BlockClassifier::class("minecraft:sand"), BlockClass::Gravity);
    assert_eq!(BlockClassifier::class("minecraft:red_concrete_powder"), BlockClass::Gravity);
    assert_eq!(BlockClassifier::class("minecraft:wall_torch"), BlockClass::Attached);
    assert_eq!(BlockClassifier::class("minecraft:ladder"), BlockClass::Attached);
    assert_eq!(BlockClassifier::class("minecraft:stone"), BlockClass::Plain);
}

#[test]
fn the_second_block_of_a_pair_is_known() {
    use stratum_core::structs::structure::{BlockState, BlockStates, StateValue};
    let block = |name: &str, key: &str, on: bool| {
        BlockState::new(name.to_string(), BlockStates::new(vec![(key.to_string(), StateValue::Bool(on))]))
    };
    assert!(BlockClassifier::is_second_half(&block("minecraft:spruce_door", "upper_block_bit", true)));
    assert!(!BlockClassifier::is_second_half(&block("minecraft:spruce_door", "upper_block_bit", false)));
    assert!(BlockClassifier::is_second_half(&block("minecraft:tall_grass", "upper_block_bit", true)));
    assert!(BlockClassifier::is_second_half(&block("minecraft:bed", "head_piece_bit", true)));
    assert!(!BlockClassifier::is_second_half(&block("minecraft:bed", "head_piece_bit", false)));
    assert!(!BlockClassifier::is_second_half(&block("minecraft:oak_trapdoor", "upper_block_bit", true)));
}

#[test]
fn near_misses_stay_plain() {
    assert_eq!(BlockClassifier::class("minecraft:sandstone"), BlockClass::Plain);
    assert_eq!(BlockClassifier::class("minecraft:red_concrete"), BlockClass::Plain);
    assert_eq!(BlockClassifier::class("minecraft:torchflower_crop"), BlockClass::Plain);
}

#[test]
fn name_families_are_matched() {
    assert_eq!(BlockClassifier::class("minecraft:anvil"), BlockClass::Gravity);
    assert_eq!(BlockClassifier::class("minecraft:suspicious_gravel"), BlockClass::Gravity);
    assert_eq!(BlockClassifier::class("minecraft:soul_torch"), BlockClass::Attached);
    assert_eq!(BlockClassifier::class("minecraft:oak_button"), BlockClass::Attached);
    assert_eq!(BlockClassifier::class("minecraft:spruce_wall_sign"), BlockClass::Attached);
    assert_eq!(BlockClassifier::class("minecraft:oak_sapling"), BlockClass::Attached);
    assert_eq!(BlockClassifier::class("minecraft:light_weighted_pressure_plate"), BlockClass::Attached);
}

#[test]
fn blocks_that_hang_from_the_block_above_are_hanging() {
    use stratum_core::structs::structure::StateValue::{Bool, Int, Text};
    let class =
        |name: &str, states: &[(&str, StateValue)]| BlockClassifier::class_of(&BlockFactory::block(name, states));
    let hanging = BlockClass::Hanging;
    assert_eq!(class("lantern", &[("hanging", Bool(true))]), hanging);
    assert_eq!(class("soul_lantern", &[("hanging", Bool(true))]), hanging);
    assert_eq!(class("lantern", &[("hanging", Bool(false))]), BlockClass::Plain);
    assert_eq!(class("oak_hanging_sign", &[("hanging", Bool(true))]), hanging);
    assert_eq!(class("oak_hanging_sign", &[]), hanging);
    assert_eq!(class("oak_hanging_sign", &[("hanging", Bool(false))]), BlockClass::Attached);
    assert_eq!(class("oak_wall_hanging_sign", &[("hanging", Bool(true))]), BlockClass::Attached);
    assert_eq!(class("wall_hanging_sign", &[]), BlockClass::Attached);
    assert_eq!(class("stone_button", &[("facing_direction", Int(0))]), hanging);
    assert_eq!(class("stone_button", &[("facing_direction", Int(1))]), BlockClass::Attached);
    assert_eq!(class("lever", &[("lever_direction", Text("down_east_west".into()))]), hanging);
    assert_eq!(class("lever", &[("lever_direction", Text("up_north_south".into()))]), BlockClass::Attached);
    assert_eq!(class("pointed_dripstone", &[("hanging", Bool(true))]), hanging);
    assert_eq!(class("pointed_dripstone", &[("hanging", Bool(false))]), BlockClass::Gravity);
    for name in ["spore_blossom", "hanging_roots", "cave_vines", "cave_vines_head_with_berries", "weeping_vines"] {
        assert_eq!(class(name, &[]), hanging, "{name}");
    }
}
