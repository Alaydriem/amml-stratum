use std::collections::BTreeMap;

use crate::structs::layout::{BigBuildLayout, LayoutIssue, PiecePlacement};
use crate::structs::library::StructureEntry;
use crate::structs::structure::StructureSize;

// Lines up the pieces of a big build from saved world origins or from grid cells.
pub struct LayoutSolver;

impl LayoutSolver {
    // The grid when every piece has a cell, else the world origins.
    pub fn solve(pieces: &[&StructureEntry]) -> Result<BigBuildLayout, LayoutIssue> {
        if !pieces.is_empty() && pieces.iter().all(|piece| piece.grid_cell.is_some()) {
            Self::from_grid(pieces)
        } else {
            Self::from_origins(pieces)
        }
    }

    pub fn from_origins(pieces: &[&StructureEntry]) -> Result<BigBuildLayout, LayoutIssue> {
        if pieces.is_empty() {
            return Err(LayoutIssue::NoPieces);
        }
        let missing: Vec<_> =
            pieces.iter().filter(|piece| piece.world_origin.is_none()).map(|piece| piece.id).collect();
        if !missing.is_empty() {
            return Err(LayoutIssue::MissingOrigin(missing));
        }
        let origins: Vec<[i64; 3]> =
            pieces.iter().filter_map(|piece| piece.world_origin).map(|origin| origin.map(i64::from)).collect();
        let mut minimum = [i64::MAX; 3];
        for origin in &origins {
            for axis in 0..3 {
                minimum[axis] = minimum[axis].min(origin[axis]);
            }
        }
        let offsets: Vec<[i64; 3]> =
            origins.iter().map(|origin| [0, 1, 2].map(|axis| origin[axis] - minimum[axis])).collect();
        Self::finish(pieces, &offsets)
    }

    pub fn from_grid(pieces: &[&StructureEntry]) -> Result<BigBuildLayout, LayoutIssue> {
        if pieces.is_empty() {
            return Err(LayoutIssue::NoPieces);
        }
        let missing: Vec<_> = pieces.iter().filter(|piece| piece.grid_cell.is_none()).map(|piece| piece.id).collect();
        if !missing.is_empty() {
            return Err(LayoutIssue::GridIncomplete(missing));
        }
        let mut widths: BTreeMap<u16, i64> = BTreeMap::new();
        let mut depths: BTreeMap<u16, i64> = BTreeMap::new();
        for piece in pieces {
            if let Some(cell) = piece.grid_cell {
                let width = widths.entry(cell.column).or_insert(0);
                *width = (*width).max(i64::from(piece.size.x));
                let depth = depths.entry(cell.row).or_insert(0);
                *depth = (*depth).max(i64::from(piece.size.z));
            }
        }
        let offsets: Vec<[i64; 3]> = pieces
            .iter()
            .filter_map(|piece| piece.grid_cell)
            .map(|cell| {
                let x = widths.range(..cell.column).map(|(_, width)| *width).sum();
                let z = depths.range(..cell.row).map(|(_, depth)| *depth).sum();
                [x, 0, z]
            })
            .collect();
        Self::finish(pieces, &offsets)
    }

    // Checks that offsets and totals fit, then that no two boxes meet, and builds the layout.
    fn finish(pieces: &[&StructureEntry], offsets: &[[i64; 3]]) -> Result<BigBuildLayout, LayoutIssue> {
        let mut extent = [0_i64; 3];
        let mut placements = Vec::with_capacity(pieces.len());
        for (piece, offset) in pieces.iter().zip(offsets) {
            let size = Self::axes(piece.size);
            let mut fitted = [0_i32; 3];
            for axis in 0..3 {
                fitted[axis] = i32::try_from(offset[axis]).map_err(|_| LayoutIssue::TooLarge)?;
                extent[axis] = extent[axis].max(offset[axis] + size[axis]);
            }
            placements.push(PiecePlacement { structure: piece.id, offset: fitted });
        }
        let mut total = [0_u32; 3];
        for axis in 0..3 {
            total[axis] = u32::try_from(extent[axis]).map_err(|_| LayoutIssue::TooLarge)?;
        }
        for (first, a) in pieces.iter().enumerate() {
            for (second, b) in pieces.iter().enumerate().skip(first + 1) {
                if Self::intersects(&offsets[first], a.size, &offsets[second], b.size) {
                    return Err(LayoutIssue::Overlap(a.id, b.id));
                }
            }
        }
        Ok(BigBuildLayout { pieces: placements, size: StructureSize::new(total[0], total[1], total[2]) })
    }

    fn intersects(a: &[i64; 3], a_size: StructureSize, b: &[i64; 3], b_size: StructureSize) -> bool {
        let a_axes = Self::axes(a_size);
        let b_axes = Self::axes(b_size);
        (0..3).all(|axis| a[axis] < b[axis] + b_axes[axis] && b[axis] < a[axis] + a_axes[axis])
    }

    fn axes(size: StructureSize) -> [i64; 3] {
        [i64::from(size.x), i64::from(size.y), i64::from(size.z)]
    }
}
