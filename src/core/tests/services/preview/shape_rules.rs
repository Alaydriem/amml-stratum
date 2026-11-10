use stratum_core::services::preview::ShapeRules;
use stratum_core::structs::block::Face;
use stratum_core::structs::preview::{BlockShape, Occlusion};
use stratum_core::structs::structure::StateValue;

use crate::support::BlockFactory;

#[test]
fn air_and_markers_of_empty_space_are_hidden() {
    for name in ["air", "structure_void", "barrier", "light_block_7"] {
        assert_eq!(ShapeRules::shape(&BlockFactory::plain(name)).0, BlockShape::Hidden, "{name}");
    }
}

#[test]
fn plants_saplings_and_torches_are_crosses_that_hide_nothing() {
    for name in ["poppy", "oak_sapling", "torch", "soul_torch", "sugar_cane", "wheat"] {
        assert_eq!(ShapeRules::shape(&BlockFactory::plain(name)), (BlockShape::Cross, Occlusion::None), "{name}");
    }
}

#[test]
fn a_slab_sits_in_the_half_its_state_names() {
    let top = BlockFactory::block("oak_slab", &[("minecraft:vertical_half", StateValue::Text("top".into()))]);
    let old_top = BlockFactory::block("stone_slab", &[("top_slot_bit", StateValue::Bool(true))]);
    let upper = BlockShape::Box { min: [0, 8, 0], max: [16, 16, 16] };
    assert_eq!(ShapeRules::shape(&top).0, upper);
    assert_eq!(ShapeRules::shape(&old_top).0, upper);
    let lower = BlockShape::Box { min: [0, 0, 0], max: [16, 8, 16] };
    assert_eq!(ShapeRules::shape(&BlockFactory::plain("oak_slab")).0, lower);
    // a double slab is a full block
    let double = ShapeRules::shape(&BlockFactory::plain("stone_double_slab"));
    assert_eq!(double, (BlockShape::Cube, Occlusion::Opaque));
}

#[test]
fn stairs_take_their_high_side_and_flip_from_their_states() {
    let sides = [(0, Face::East), (1, Face::West), (2, Face::South), (3, Face::North)];
    for (direction, high_side) in sides {
        let stairs = BlockFactory::block(
            "oak_stairs",
            &[("weirdo_direction", StateValue::Int(direction)), ("upside_down_bit", StateValue::Int(1))],
        );
        assert_eq!(ShapeRules::shape(&stairs).0, BlockShape::Stairs { upside_down: true, high_side });
    }
}

#[test]
fn thin_blocks_are_boxes_that_hide_nothing() {
    let flat = BlockShape::Box { min: [0, 0, 0], max: [16, 1, 16] };
    for name in ["white_carpet", "rail", "redstone_wire", "stone_pressure_plate"] {
        assert_eq!(ShapeRules::shape(&BlockFactory::plain(name)), (flat, Occlusion::None), "{name}");
    }
    let post = BlockShape::Box { min: [6, 0, 6], max: [10, 16, 10] };
    assert_eq!(ShapeRules::shape(&BlockFactory::plain("oak_fence")).0, post);
    let pane = BlockShape::Box { min: [7, 0, 7], max: [9, 16, 9] };
    assert_eq!(ShapeRules::shape(&BlockFactory::plain("glass_pane")).0, pane);
}

#[test]
fn see_through_blocks_hide_only_their_own_kind() {
    let (shape, glass) = ShapeRules::shape(&BlockFactory::plain("glass"));
    assert_eq!(shape, BlockShape::Cube);
    assert!(matches!(glass, Occlusion::Group(_)));
    let still = ShapeRules::shape(&BlockFactory::plain("water")).1;
    let flowing = ShapeRules::shape(&BlockFactory::plain("flowing_water")).1;
    assert_eq!(still, flowing);
    assert_ne!(ShapeRules::shape(&BlockFactory::plain("ice")).1, glass);
    assert_eq!(ShapeRules::shape(&BlockFactory::plain("oak_leaves")), (BlockShape::Cube, Occlusion::None));
    assert_eq!(ShapeRules::shape(&BlockFactory::plain("stone")), (BlockShape::Cube, Occlusion::Opaque));
}
