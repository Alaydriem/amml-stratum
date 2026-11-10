use stratum_core::services::{ItemRules, MaterialCounter, StructureParser};
use stratum_core::structs::material::MaterialList;
use stratum_core::structs::nbt::NbtTag;
use stratum_core::structs::structure::{BlockLayer, Structure};

use crate::support::{BlockFactory, StructureFixture};

#[test]
fn counter_counts_a_door_once() {
    let bytes = StructureFixture::new([1, 2, 1])
        .block(0, 0, 0, "minecraft:oak_door")
        .state("upper_block_bit", NbtTag::Byte(0))
        .block(0, 1, 0, "minecraft:oak_door")
        .state("upper_block_bit", NbtTag::Byte(1))
        .bytes();
    let structure = StructureParser::parse(&bytes).unwrap();
    let list = MaterialCounter::count(&ItemRules::new(), &structure);

    assert_eq!(list.lines().len(), 1);
    assert_eq!(list.lines()[0].item, BlockFactory::item("oak_door"));
    assert_eq!(list.lines()[0].count, 1);
    assert_eq!(list.total(), 1);
}

#[test]
fn counter_merges_palette_entries_that_share_an_item() {
    let bytes = StructureFixture::new([3, 1, 1])
        .block(0, 0, 0, "minecraft:candle")
        .state("candles", NbtTag::Int(1))
        .block(1, 0, 0, "minecraft:candle")
        .state("candles", NbtTag::Int(3))
        .block(2, 0, 0, "minecraft:lit_furnace")
        .bytes();
    let structure = StructureParser::parse(&bytes).unwrap();
    let list = MaterialCounter::count(&ItemRules::new(), &structure);

    assert_eq!(list.lines().len(), 2);
    assert_eq!(list.lines()[0].item, BlockFactory::item("candle"));
    assert_eq!(list.lines()[0].count, 6);
    assert_eq!(list.lines()[1].item, BlockFactory::item("furnace"));
    assert_eq!(list.lines()[1].count, 1);
}

#[test]
fn counter_reports_stack_sizes() {
    let bytes = StructureFixture::new([1, 1, 1]).block(0, 0, 0, "minecraft:oak_sign").bytes();
    let structure = StructureParser::parse(&bytes).unwrap();
    let list = MaterialCounter::count(&ItemRules::new(), &structure);

    assert_eq!(list.lines()[0].stack_size, 16);
}

#[test]
fn counter_ignores_waterlogging() {
    // The water sits in the secondary layer, as a waterlogged cell stores it.
    let structure = StructureParser::parse(
        &StructureFixture::new([1, 1, 2])
            .block(0, 0, 0, "minecraft:stone")
            .waterlog(0, 0, 1, "minecraft:water")
            .state("liquid_depth", NbtTag::Int(0))
            .bytes(),
    )
    .unwrap();
    assert_eq!(structure.secondary.len(), 1);
    let list = MaterialCounter::count(&ItemRules::new(), &structure);

    assert_eq!(list.lines().len(), 1);
    assert_eq!(list.lines()[0].item, BlockFactory::item("stone"));
}

#[test]
fn counter_skips_empty_cells_and_bad_indices() {
    let parsed =
        StructureParser::parse(&StructureFixture::new([3, 1, 1]).block(0, 0, 0, "minecraft:stone").bytes()).unwrap();
    // A layer built in code may point past the palette; the parser never makes one.
    let structure = Structure { primary: BlockLayer::new(vec![0, 99, BlockLayer::NONE]), ..parsed };
    let list = MaterialCounter::count(&ItemRules::new(), &structure);

    assert_eq!(list.total(), 1);
}

#[test]
fn list_sorts_by_count_then_id() {
    let list = MaterialList::new(vec![
        BlockFactory::line("b", 5),
        BlockFactory::line("c", 9),
        BlockFactory::line("a", 5),
        BlockFactory::line("d", 1),
    ]);
    let order: Vec<&str> = list.lines().iter().map(|entry| entry.item.0.as_str()).collect();

    assert_eq!(order, ["minecraft:c", "minecraft:a", "minecraft:b", "minecraft:d"]);
    assert_eq!(list.total(), 20);
}

#[test]
fn list_folds_duplicate_items() {
    let list = MaterialList::new(vec![
        BlockFactory::line("stone", 3),
        BlockFactory::line("dirt", 5),
        BlockFactory::line("stone", 4),
    ]);
    let pairs: Vec<(&str, u64)> = list.lines().iter().map(|entry| (entry.item.0.as_str(), entry.count)).collect();

    assert_eq!(pairs, [("minecraft:stone", 7), ("minecraft:dirt", 5)]);
}

#[test]
fn merge_adds_and_resorts() {
    let first = MaterialList::new(vec![BlockFactory::line("stone", 10), BlockFactory::line("dirt", 4)]);
    let second = MaterialList::new(vec![BlockFactory::line("dirt", 8), BlockFactory::line("glass", 3)]);
    let merged = MaterialList::merge(&[&first, &second]);
    let pairs: Vec<(&str, u64)> = merged.lines().iter().map(|entry| (entry.item.0.as_str(), entry.count)).collect();

    assert_eq!(pairs, [("minecraft:dirt", 12), ("minecraft:stone", 10), ("minecraft:glass", 3)]);
    assert_eq!(merged.total(), 25);
}

#[test]
fn merge_of_nothing_is_empty() {
    let merged = MaterialList::merge(&[]);

    assert!(merged.lines().is_empty());
    assert_eq!(merged.total(), 0);
}

#[test]
fn readable_names() {
    assert_eq!(BlockFactory::item("stone_bricks").readable(), "Stone Bricks");
    assert_eq!(BlockFactory::item("oak_sign").readable(), "Oak Sign");
}
