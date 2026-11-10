use stratum_core::services::{ItemRules, PlaceNeeds};
use stratum_core::structs::easy_place::{FaceRule, Half, PlaceNeed};
use stratum_core::structs::ghost::GhostPalette;
use stratum_core::structs::material::ItemYield;
use stratum_core::structs::structure::{BlockState, StateValue};

use crate::support::BlockFactory;

struct Needs;

impl Needs {
    fn of(blocks: Vec<BlockState>) -> PlaceNeeds {
        let palette = GhostPalette { entries: blocks };
        PlaceNeeds::new(&palette, &ItemRules::new())
    }

    fn place(item: &str, rule: FaceRule) -> PlaceNeed {
        PlaceNeed::Place { item: BlockFactory::item(item), rule, slab_of: None, half: Half::Any }
    }

    fn rules(needs: &PlaceNeeds, count: u16) -> Vec<FaceRule> {
        (0..count)
            .map(|id| match needs.need(id) {
                PlaceNeed::Place { rule, .. } => *rule,
                PlaceNeed::NotTarget => panic!("id {id} is not a target"),
            })
            .collect()
    }
}

#[test]
fn liquids_and_upper_halves_are_not_targets() {
    let upper = [("upper_block_bit", StateValue::Bool(true))];
    let lower = [("upper_block_bit", StateValue::Bool(false))];
    let lower_door = BlockFactory::block("oak_door", &lower);
    let expected = match ItemRules::new().items_for(&lower_door) {
        ItemYield::One(item, _) => item,
        other => panic!("unexpected yield {other:?}"),
    };
    let needs = Needs::of(vec![
        BlockFactory::plain("water"),
        BlockFactory::plain("lava"),
        BlockFactory::plain("flowing_water"),
        BlockFactory::plain("flowing_lava"),
        BlockFactory::block("oak_door", &upper),
        lower_door,
    ]);
    for id in 0..5 {
        assert_eq!(*needs.need(id), PlaceNeed::NotTarget, "id {id}");
    }
    assert_eq!(
        *needs.need(5),
        PlaceNeed::Place { item: expected, rule: FaceRule::Any, slab_of: None, half: Half::Any }
    );
}

#[test]
fn a_double_slab_needs_its_slab_twice() {
    let needs = Needs::of(vec![BlockFactory::plain("oak_double_slab"), BlockFactory::plain("oak_slab")]);
    assert_eq!(
        *needs.need(0),
        PlaceNeed::Place {
            item: BlockFactory::item("oak_slab"),
            rule: FaceRule::Any,
            slab_of: Some("minecraft:oak_slab".to_string()),
            half: Half::Any,
        }
    );
    assert_eq!(*needs.need(1), Needs::place("oak_slab", FaceRule::Any));
}

#[test]
fn wall_torches_need_a_wall_and_torches_a_floor() {
    let needs = Needs::of(vec![
        BlockFactory::plain("wall_torch"),
        BlockFactory::plain("soul_wall_torch"),
        BlockFactory::plain("torch"),
        BlockFactory::plain("soul_torch"),
        BlockFactory::plain("unlit_redstone_torch"),
        BlockFactory::plain("ladder"),
        BlockFactory::plain("stone"),
        BlockFactory::plain("redstone_wall_torch"),
        BlockFactory::plain("redstone_torch"),
    ]);
    assert_eq!(
        Needs::rules(&needs, 9),
        vec![
            FaceRule::Wall,
            FaceRule::Wall,
            FaceRule::Floor,
            FaceRule::Floor,
            FaceRule::Floor,
            FaceRule::Wall,
            FaceRule::Any,
            FaceRule::Wall,
            FaceRule::Floor,
        ]
    );
}

#[test]
fn wall_signs_need_a_wall() {
    let needs = Needs::of(vec![
        BlockFactory::plain("wall_sign"),
        BlockFactory::plain("oak_wall_sign"),
        BlockFactory::plain("oak_wall_hanging_sign"),
        BlockFactory::plain("standing_sign"),
        BlockFactory::plain("oak_standing_sign"),
        BlockFactory::plain("wall_banner"),
        BlockFactory::plain("standing_banner"),
        BlockFactory::plain("tube_wall_fan"),
    ]);
    assert_eq!(
        Needs::rules(&needs, 8),
        vec![
            FaceRule::Wall,
            FaceRule::Wall,
            FaceRule::Wall,
            FaceRule::Floor,
            FaceRule::Floor,
            FaceRule::Wall,
            FaceRule::Floor,
            FaceRule::Wall,
        ]
    );
    assert_eq!(*needs.need(0), Needs::place("oak_sign", FaceRule::Wall));
}

#[test]
fn the_marker_entry_is_not_a_target() {
    let needs = Needs::of(vec![BlockFactory::plain("stone"), GhostPalette::marker_entry()]);
    assert_eq!(*needs.need(1), PlaceNeed::NotTarget);
    assert_eq!(needs.block_name(1), Some("minecraft:structure_block"));
}

#[test]
fn an_unknown_palette_id_is_not_a_target() {
    let needs = Needs::of(vec![BlockFactory::plain("stone")]);
    assert_eq!(*needs.need(1), PlaceNeed::NotTarget);
    assert_eq!(*needs.need(u16::MAX), PlaceNeed::NotTarget);
    assert_eq!(needs.block_name(1), None);
    assert_eq!(needs.block_name(0), Some("minecraft:stone"));
}

#[test]
fn a_block_made_of_two_items_needs_the_first() {
    let needs = Needs::of(vec![BlockFactory::plain("candle_cake")]);
    assert_eq!(*needs.need(0), Needs::place("cake", FaceRule::Any));
}

#[test]
fn slab_halves_and_upside_down_stairs_give_a_half() {
    let top = [("minecraft:vertical_half", StateValue::Text("top".to_string()))];
    let bottom = [("minecraft:vertical_half", StateValue::Text("bottom".to_string()))];
    let upside_down = [("upside_down_bit", StateValue::Bool(true))];
    let upright = [("upside_down_bit", StateValue::Int(0))];
    let needs = Needs::of(vec![
        BlockFactory::block("oak_slab", &top),
        BlockFactory::block("oak_slab", &bottom),
        BlockFactory::block("oak_stairs", &upside_down),
        BlockFactory::block("oak_stairs", &upright),
        BlockFactory::block("oak_double_slab", &bottom),
        BlockFactory::plain("stone"),
    ]);
    let halves: Vec<Half> = (0..6)
        .map(|id| match needs.need(id) {
            PlaceNeed::Place { half, .. } => *half,
            PlaceNeed::NotTarget => panic!("id {id} is not a target"),
        })
        .collect();
    assert_eq!(halves, vec![Half::Top, Half::Bottom, Half::Top, Half::Bottom, Half::Any, Half::Any]);
}
