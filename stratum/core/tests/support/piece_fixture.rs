use stratum_core::structs::layout::{GridCell, PiecePlacement};
use stratum_core::structs::library::{StructureEntry, StructureId};
use stratum_core::structs::structure::StructureSize;

pub struct PieceFixture;

impl PieceFixture {
    pub fn offsets(pieces: &[PiecePlacement]) -> Vec<(u64, [i32; 3])> {
        pieces.iter().map(|p| (p.structure.0, p.offset)).collect()
    }

    pub fn at(id: u64, origin: [i32; 3], size: [u32; 3]) -> StructureEntry {
        StructureEntry {
            id: StructureId(id),
            display_name: format!("piece {id}"),
            file_name: format!("piece{id}.mcstructure"),
            project: None,
            size: StructureSize::new(size[0], size[1], size[2]),
            block_count: 1,
            world_origin: Some(origin),
            grid_cell: None,
        }
    }

    pub fn without_origin(id: u64, size: [u32; 3]) -> StructureEntry {
        StructureEntry { world_origin: None, ..Self::at(id, [0, 0, 0], size) }
    }

    pub fn in_cell(id: u64, column: u16, row: u16, size: [u32; 3]) -> StructureEntry {
        StructureEntry { grid_cell: Some(GridCell::new(column, row)), ..Self::without_origin(id, size) }
    }
}
