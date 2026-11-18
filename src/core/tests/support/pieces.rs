use std::sync::Arc;

use stratum_core::structs::ghost::GhostPiece;
use stratum_core::structs::structure::{
    BlockLayer, BlockState, BlockStates, SparseLayer, StateValue, Structure, StructureSize,
};

// Builds build pieces from text. Each string is one row along x, the rows of a layer run along z, and the
// layers run along y. '#' stone, '.' air, 'w' water, 's' sand, 't' wall torch, 'v' structure void, 'd' dirt,
// 'L' and 'U' the lower and upper half of an oak door, 'h' a lantern that hangs from the block above.
pub struct Pieces;

impl Pieces {
    const KEYS: [(char, &'static str); 10] = [
        ('#', "stone"),
        ('.', "air"),
        ('w', "water"),
        ('s', "sand"),
        ('t', "wall_torch"),
        ('v', "structure_void"),
        ('d', "dirt"),
        ('L', "wooden_door"),
        ('U', "wooden_door"),
        ('h', "lantern"),
    ];

    pub fn layer(rows: &[&str]) -> GhostPiece {
        Self::layer_at(rows, [0, 0, 0])
    }

    pub fn layer_at(rows: &[&str], world_min: [i32; 3]) -> GhostPiece {
        Self::layers_at(&[rows], world_min)
    }

    pub fn layers(layers: &[&[&str]]) -> GhostPiece {
        Self::layers_at(layers, [0, 0, 0])
    }

    pub fn layers_at(layers: &[&[&str]], world_min: [i32; 3]) -> GhostPiece {
        let size_x = layers.first().and_then(|rows| rows.first()).map_or(0, |row| row.len());
        let size_z = layers.first().map_or(0, |rows| rows.len());
        let size = StructureSize::new(size_x as u32, layers.len() as u32, size_z as u32);
        let mut cells = vec![BlockLayer::NONE; size_x * layers.len() * size_z];
        for (y, rows) in layers.iter().enumerate() {
            for (z, row) in rows.iter().enumerate() {
                for (x, key) in row.chars().enumerate() {
                    let palette = Self::KEYS.iter().position(|(known, _)| *known == key).expect("a known key");
                    cells[(x * layers.len() + y) * size_z + z] = palette as u16;
                }
            }
        }
        let structure = Structure {
            size,
            world_origin: None,
            palette: Self::KEYS
                .iter()
                .map(|(key, name)| BlockState::new(format!("minecraft:{name}"), Self::states(*key)))
                .collect(),
            primary: BlockLayer::new(cells),
            secondary: SparseLayer::default(),
            block_entities: Default::default(),
        };
        GhostPiece { structure: Arc::new(structure), world_min, remap: Vec::new() }
    }

    fn states(key: char) -> BlockStates {
        match key {
            'L' | 'U' => BlockStates::new(vec![("upper_block_bit".to_string(), StateValue::Bool(key == 'U'))]),
            'h' => BlockStates::new(vec![("hanging".to_string(), StateValue::Bool(true))]),
            _ => BlockStates::default(),
        }
    }

    // A skip list that marks dirt as not to place.
    pub fn skip_dirt() -> Vec<bool> {
        Self::KEYS.iter().map(|(key, _)| *key == 'd').collect()
    }
}
