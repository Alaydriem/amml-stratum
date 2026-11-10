use std::collections::HashMap;
use std::sync::Arc;

use stratum_core::structs::ghost::{GhostBuild, GhostPalette, GhostPiece};
use stratum_core::structs::library::{Library, Project, ProjectId, ProjectKind, StructureEntry, StructureId};
use stratum_core::structs::marker::{Marker, MarkerId, MarkerTarget, MarkerView, Rotation};
use stratum_core::structs::structure::{BlockLayer, BlockState, BlockStates, SparseLayer, Structure, StructureSize};

use super::{BlockFactory, PieceFixture};

// Builds structures, markers and libraries for ghost build tests.
pub struct GhostFixture;

impl GhostFixture {
    // A structure whose primary layer lists `names` in cell order. An empty name is no block.
    pub fn structure(size: [u32; 3], names: &[&str]) -> Arc<Structure> {
        let mut palette: Vec<BlockState> = Vec::new();
        let mut cells = Vec::with_capacity(names.len());
        for name in names {
            if name.is_empty() {
                cells.push(BlockLayer::NONE);
                continue;
            }
            let block = BlockFactory::plain(name);
            let index = palette.iter().position(|b| *b == block).unwrap_or_else(|| {
                palette.push(block);
                palette.len() - 1
            });
            cells.push(index as u16);
        }
        Arc::new(Structure {
            size: StructureSize::new(size[0], size[1], size[2]),
            world_origin: None,
            palette,
            primary: BlockLayer::new(cells),
            secondary: SparseLayer::default(),
        })
    }

    // A 1 x 1 x 1 structure holding one block with its states.
    pub fn stateful(block: BlockState) -> Arc<Structure> {
        Arc::new(Structure {
            size: StructureSize::new(1, 1, 1),
            world_origin: None,
            palette: vec![block],
            primary: BlockLayer::new(vec![0]),
            secondary: SparseLayer::default(),
        })
    }

    // A 1 x 1 x `count` structure of `count` different blocks.
    pub fn distinct(count: usize) -> Arc<Structure> {
        let palette: Vec<BlockState> =
            (0..count).map(|n| BlockState::new(format!("test:block_{n}"), BlockStates::default())).collect();
        Arc::new(Structure {
            size: StructureSize::new(1, 1, count as u32),
            world_origin: None,
            palette,
            primary: BlockLayer::new((0..count as u16).collect()),
            secondary: SparseLayer::default(),
        })
    }

    // A build of one piece holding stone in every cell of each inclusive world box.
    pub fn blocks(boxes: &[([i32; 3], [i32; 3])]) -> GhostBuild {
        let mut min = boxes[0].0;
        let mut max = boxes[0].1;
        for (low, high) in boxes {
            for axis in 0..3 {
                min[axis] = min[axis].min(low[axis]);
                max[axis] = max[axis].max(high[axis]);
            }
        }
        let size = [0, 1, 2].map(|axis| (max[axis] - min[axis] + 1) as u32);
        let (sy, sz) = (size[1] as usize, size[2] as usize);
        let mut cells = vec![BlockLayer::NONE; size[0] as usize * sy * sz];
        for (low, high) in boxes {
            for x in low[0]..=high[0] {
                for y in low[1]..=high[1] {
                    for z in low[2]..=high[2] {
                        let (lx, ly, lz) = ((x - min[0]) as usize, (y - min[1]) as usize, (z - min[2]) as usize);
                        cells[(lx * sy + ly) * sz + lz] = 0;
                    }
                }
            }
        }
        let mut palette = GhostPalette { entries: vec![BlockFactory::plain("stone")] };
        let marker_block = palette.marker_id().unwrap();
        let structure = Arc::new(Structure {
            size: StructureSize::new(size[0], size[1], size[2]),
            world_origin: None,
            palette: palette.entries.clone(),
            primary: BlockLayer::new(cells),
            secondary: SparseLayer::default(),
        });
        GhostBuild {
            pieces: vec![GhostPiece { structure, world_min: min, remap: vec![0] }],
            palette,
            marker_block,
            world_min: min,
            world_max: max,
            revision: 1,
        }
    }

    pub fn marker(origin: [i32; 3], target: MarkerTarget) -> Marker {
        Marker {
            id: MarkerId(1),
            name: "marker".to_string(),
            origin,
            rotation: Rotation::R0,
            target,
            view: MarkerView::default(),
        }
    }

    pub fn loaded(structures: &[(u64, Arc<Structure>)]) -> HashMap<StructureId, Arc<Structure>> {
        structures.iter().map(|(id, structure)| (StructureId(*id), structure.clone())).collect()
    }

    // A library with one project (id 1) holding pieces placed at their world origins.
    pub fn library(kind: ProjectKind, pieces: &[(u64, [i32; 3], [u32; 3])]) -> Library {
        let mut library = Library::new();
        library.projects.push(Project { id: ProjectId(1), name: "project".to_string(), kind });
        for (id, origin, size) in pieces {
            let entry = StructureEntry { project: Some(ProjectId(1)), ..PieceFixture::at(*id, *origin, *size) };
            library.structures.push(entry);
        }
        library
    }
}
