use stratum_core::services::CommandText;
use stratum_core::structs::build::{ClearPart, PaletteOutcome};
use stratum_core::structs::structure::StateValue;

use crate::support::BlockFactory;

#[test]
fn set_block_with_states() {
    let stair = BlockFactory::block(
        "oak_stairs",
        &[("weirdo_direction", StateValue::Int(2)), ("upside_down_bit", StateValue::Bool(true))],
    );
    assert_eq!(
        CommandText::set_block([-30_000_123, -60, 7], &stair, PaletteOutcome::Exact),
        r#"/setblock -30000123 -60 7 minecraft:oak_stairs ["weirdo_direction"=2,"upside_down_bit"=true]"#
    );
}

#[test]
fn set_block_name_only_and_no_states() {
    let stone = BlockFactory::plain("stone");
    assert_eq!(CommandText::set_block([0, 0, 0], &stone, PaletteOutcome::Exact), "/setblock 0 0 0 minecraft:stone");
    let log = BlockFactory::block("oak_log", &[("pillar_axis", StateValue::Text("x".into()))]);
    assert_eq!(CommandText::set_block([0, 0, 0], &log, PaletteOutcome::NameOnly), "/setblock 0 0 0 minecraft:oak_log");
}

#[test]
fn unknown_outcome_reads_like_exact() {
    let log = BlockFactory::block("oak_log", &[("pillar_axis", StateValue::Text("x".into()))]);
    assert_eq!(
        CommandText::set_block([1, 2, 3], &log, PaletteOutcome::Unknown),
        CommandText::set_block([1, 2, 3], &log, PaletteOutcome::Exact)
    );
}

#[test]
fn text_values_are_quoted_and_escaped() {
    let odd = BlockFactory::namespaced_with("mymod:odd", &[("label", StateValue::Text("a \"b\" c".into()))]);
    assert_eq!(
        CommandText::set_block([0, 0, 0], &odd, PaletteOutcome::Exact),
        r#"/setblock 0 0 0 mymod:odd ["label"="a \"b\" c"]"#
    );
    let slash = BlockFactory::namespaced_with("mymod:odd", &[("label", StateValue::Text(r"a\b".into()))]);
    assert_eq!(
        CommandText::set_block([0, 0, 0], &slash, PaletteOutcome::Exact),
        r#"/setblock 0 0 0 mymod:odd ["label"="a\\b"]"#
    );
}

#[test]
fn fills() {
    let all = ClearPart { min: [-5, -64, -5], max: [4, -60, 4], kind: None };
    assert_eq!(CommandText::fill_air(&all), "/fill -5 -64 -5 4 -60 4 air");
    let one = ClearPart { kind: Some("minecraft:stone".into()), ..all };
    assert_eq!(CommandText::fill_air(&one), "/fill -5 -64 -5 4 -60 4 air replace minecraft:stone");
}

#[test]
fn game_rule() {
    assert_eq!(CommandText::game_rule("sendcommandfeedback", false), "/gamerule sendcommandfeedback false");
}
