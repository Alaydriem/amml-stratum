use std::sync::Arc;

use stratum_core::services::NbtWriter;
use stratum_core::structs::ghost::{ChestPairNbt, GhostBlockEntityTable, GhostPiece};
use stratum_core::structs::nbt::NbtTag;
use stratum_core::structs::structure::{
    BlockEntities, BlockLayer, BlockState, BlockStates, SparseLayer, Structure, StructureSize,
};

// A piece of `nbt.len()` cells along X, cell `n` holding NBT `n`.
fn piece(nbt: Vec<Vec<u8>>) -> GhostPiece {
    let count = nbt.len() as u32;
    let mut block_entities = BlockEntities::new();
    for (cell, bytes) in nbt.into_iter().enumerate() {
        block_entities.insert(cell as u32, bytes);
    }
    let structure = Structure {
        size: StructureSize::new(count, 1, 1),
        world_origin: None,
        palette: vec![BlockState::new("minecraft:chest".to_string(), BlockStates::default())],
        primary: BlockLayer::new(vec![0; count as usize]),
        secondary: SparseLayer::default(),
        block_entities,
    };
    GhostPiece { structure: Arc::new(structure), world_min: [0; 3], remap: vec![0] }
}

fn chest(pair: Option<(i32, i32, Option<i8>)>) -> Vec<u8> {
    let mut tags = vec![("id", NbtTag::String("Chest".to_string()))];
    if let Some((x, z, lead)) = pair {
        tags.push(("pairx", NbtTag::Int(x)));
        tags.push(("pairz", NbtTag::Int(z)));
        if let Some(lead) = lead {
            tags.push(("pairlead", NbtTag::Byte(lead)));
        }
    }
    NbtWriter::compound("", &tags).unwrap()
}

#[test]
fn the_pair_keys_of_each_entry_are_kept_beside_the_stripped_table() {
    let first = piece(vec![chest(Some((1, 0, Some(1)))), chest(None)]);
    let second = piece(vec![chest(Some((-7, 3, Some(0)))), chest(Some((4, -2, None)))]);

    let table = GhostBlockEntityTable::collect(&[first, second]);

    assert_eq!(table.len(), 1, "every chest strips to the same bytes");
    assert_eq!(table.chest_pair(0), Some(ChestPairNbt::new(1, 0, true)));
    assert_eq!(table.chest_pair(1), None, "no pair keys");
    assert_eq!(table.chest_pair(2), Some(ChestPairNbt::new(-7, 3, false)));
    assert_eq!(table.chest_pair(3), Some(ChestPairNbt::new(4, -2, false)), "no pairlead is not a lead");
    assert_eq!(table.chest_pair(4), None, "past the last entry");
}
