use std::collections::HashSet;

use amml_api::{BlockPos, BlockSource, KindsInBox};
use stratum_core::structs::build::{BuildKindsRequest, BuildKindsResult};

use crate::traits::BlockGrid;

/// Reads the block kinds of a box of the world for the core's clear, one slab of at most `cells_per_tick` cells a
/// tick (never more than the loader's `BlockSource::KINDS_CELL_CAP`). A slab with a chunk that is not loaded, or one
/// the loader refuses, answers `NotLoaded`; a box with more kinds than `KIND_BOUND` answers `TooMany`.
///
/// Bound: at most `KIND_BOUND` kinds and names are held; they are dropped when the answer is given.
pub struct KindsScanner {
    cells_per_tick: u32,
    active: bool,
    min: [i32; 3],
    max: [i32; 3],
    // the minimum corner of the next slab
    cursor: [i32; 3],
    // the slab size on x, y and z, set at the start of a scan
    slab: [i32; 3],
    seen: HashSet<String>,
    names: Vec<String>,
}

impl KindsScanner {
    /// The most kinds one scan holds. The core keeps its own, smaller, limit from its config.
    pub const KIND_BOUND: usize = 16_384;

    /// The cells one tick reads when the config has no other rate: one full loader box.
    pub const CELLS_PER_TICK: u32 = BlockSource::KINDS_CELL_CAP;

    pub fn new(cells_per_tick: u32) -> Self {
        Self {
            cells_per_tick: cells_per_tick.clamp(1, BlockSource::KINDS_CELL_CAP),
            active: false,
            min: [0; 3],
            max: [0; 3],
            cursor: [0; 3],
            slab: [1; 3],
            seen: HashSet::new(),
            names: Vec::new(),
        }
    }

    /// Starts reading the box of `request`, in place of a scan that is running.
    pub fn begin(&mut self, request: &BuildKindsRequest) {
        self.clear();
        self.active = true;
        self.min = request.min;
        self.max = request.max;
        self.cursor = request.min;
        self.slab = self.slab_size();
    }

    pub fn is_scanning(&self) -> bool {
        self.active
    }

    /// The kinds found so far.
    pub fn found(&self) -> usize {
        self.names.len()
    }

    /// Reads one slab; the answer when the box is done or cannot be read, None while slabs are left.
    pub fn step(&mut self, grid: &dyn BlockGrid) -> Option<BuildKindsResult> {
        if !self.active {
            return None;
        }
        if self.min.iter().zip(&self.max).any(|(low, high)| low > high) {
            return Some(self.finish(BuildKindsResult::Kinds(Vec::new())));
        }
        let low = self.cursor;
        let high: [i32; 3] = std::array::from_fn(|axis| (low[axis] + self.slab[axis] - 1).min(self.max[axis]));
        let read = grid.kinds_in(
            BlockPos { x: low[0], y: low[1], z: low[2] },
            BlockPos { x: high[0], y: high[1], z: high[2] },
            Self::KIND_BOUND as u32,
        );
        match read {
            Ok(KindsInBox::Kinds(kinds)) => {
                for kind in kinds {
                    if self.seen.contains(&kind) {
                        continue;
                    }
                    if self.names.len() >= Self::KIND_BOUND {
                        return Some(self.finish(BuildKindsResult::TooMany));
                    }
                    self.seen.insert(kind.clone());
                    self.names.push(kind);
                }
            }
            Ok(KindsInBox::TooMany) => return Some(self.finish(BuildKindsResult::TooMany)),
            Ok(KindsInBox::NotLoaded) | Err(_) => return Some(self.finish(BuildKindsResult::NotLoaded)),
        }
        if !self.advance() {
            let kinds = std::mem::take(&mut self.names);
            return Some(self.finish(BuildKindsResult::Kinds(kinds)));
        }
        None
    }

    // the largest slab of at most `cells_per_tick` cells: full columns first, then rows of z, then planes of x
    fn slab_size(&self) -> [i32; 3] {
        let side = |axis: usize| (i64::from(self.max[axis]) - i64::from(self.min[axis]) + 1).max(1);
        let cap = i64::from(self.cells_per_tick);
        let y = side(1).min(cap);
        let z = side(2).min((cap / y).max(1));
        let x = side(0).min((cap / (y * z)).max(1));
        [x, y, z].map(|size| i32::try_from(size).unwrap_or(i32::MAX))
    }

    // moves the cursor to the next slab, up the column first, then along z, then along x; false at the end of the box
    fn advance(&mut self) -> bool {
        for axis in [1usize, 2, 0] {
            let next = self.cursor[axis].saturating_add(self.slab[axis]);
            if next <= self.max[axis] {
                self.cursor[axis] = next;
                return true;
            }
            self.cursor[axis] = self.min[axis];
        }
        false
    }

    fn finish(&mut self, answer: BuildKindsResult) -> BuildKindsResult {
        self.clear();
        answer
    }

    fn clear(&mut self) {
        self.active = false;
        self.seen.clear();
        self.names.clear();
    }
}
