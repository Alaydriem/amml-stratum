use std::collections::HashSet;

use stratum_core::structs::build::{BuildKindsRequest, BuildKindsResult};

use crate::traits::BlockGrid;

/// Reads the block kinds of a box of the world for the core's clear, a fixed number of cells a tick. A box with a
/// chunk that is not loaded, a block that reads null or a kind whose name cannot be read answers `NotLoaded`; a box
/// with more kinds than `KIND_BOUND` answers `TooMany`.
///
/// Bound: at most `KIND_BOUND` kinds and names are held; they are dropped when the answer is given.
pub struct KindsScanner {
    cells_per_tick: usize,
    active: bool,
    min: [i32; 3],
    max: [i32; 3],
    cursor: [i32; 3],
    // the chunk of the last cell read, so a chunk is asked for once per run of cells in it
    chunk: Option<(i32, i32)>,
    // the kind of the last cell read, so a run of the same kind is looked up once
    last_kind: Option<usize>,
    // whether the unreadable name of this scan was logged
    name_logged: bool,
    seen: HashSet<usize>,
    names: Vec<String>,
}

impl KindsScanner {
    /// The most kinds one scan holds. The core keeps its own, smaller, limit from its config.
    pub const KIND_BOUND: usize = 16_384;

    /// The cells one tick reads when the config has no other rate.
    pub const CELLS_PER_TICK: usize = 65_536;

    // an `i32` shifted by 4 floors to the chunk coordinate, for negative cells too
    const CHUNK_SHIFT: u32 = 4;

    pub fn new(cells_per_tick: usize) -> Self {
        Self {
            cells_per_tick: cells_per_tick.max(1),
            active: false,
            min: [0; 3],
            max: [0; 3],
            cursor: [0; 3],
            chunk: None,
            last_kind: None,
            name_logged: false,
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
    }

    pub fn is_scanning(&self) -> bool {
        self.active
    }

    /// The kinds found so far.
    pub fn found(&self) -> usize {
        self.names.len()
    }

    /// Reads up to a tick's cells; the answer when the box is done or cannot be read, None while cells are left.
    pub fn step(&mut self, grid: &dyn BlockGrid) -> Option<BuildKindsResult> {
        if !self.active {
            return None;
        }
        if self.min.iter().zip(&self.max).any(|(low, high)| low > high) {
            return Some(self.finish(BuildKindsResult::Kinds(Vec::new())));
        }
        // a chunk can unload between ticks, so each tick asks again
        self.chunk = None;
        for _ in 0..self.cells_per_tick {
            if let Some(refusal) = self.read_cell(grid) {
                return Some(self.finish(refusal));
            }
            if !self.advance() {
                let kinds = std::mem::take(&mut self.names);
                return Some(self.finish(BuildKindsResult::Kinds(kinds)));
            }
        }
        None
    }

    // reads the cell under the cursor; a refusal ends the scan
    fn read_cell(&mut self, grid: &dyn BlockGrid) -> Option<BuildKindsResult> {
        let [x, y, z] = self.cursor;
        let chunk = (x >> Self::CHUNK_SHIFT, z >> Self::CHUNK_SHIFT);
        if self.chunk != Some(chunk) {
            if !grid.chunk_loaded(chunk.0, chunk.1) {
                return Some(BuildKindsResult::NotLoaded);
            }
            self.chunk = Some(chunk);
        }
        let Some(kind) = grid.kind_at([x, y, z]) else {
            return Some(BuildKindsResult::NotLoaded);
        };
        if self.last_kind == Some(kind) || self.seen.contains(&kind) {
            self.last_kind = Some(kind);
            return None;
        }
        if self.names.len() >= Self::KIND_BOUND {
            return Some(BuildKindsResult::TooMany);
        }
        let Some(name) = grid.kind_name(kind) else {
            if !self.name_logged {
                self.name_logged = true;
                curia::warn!("a block type name could not be read, the scan stops", { kind: kind });
            }
            return Some(BuildKindsResult::NotLoaded);
        };
        self.seen.insert(kind);
        self.names.push(name);
        self.last_kind = Some(kind);
        None
    }

    // moves the cursor to the next cell, up the column first, then along z, then along x; false at the end of the box
    fn advance(&mut self) -> bool {
        for axis in [1usize, 2, 0] {
            if self.cursor[axis] < self.max[axis] {
                self.cursor[axis] += 1;
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
        self.chunk = None;
        self.last_kind = None;
        self.name_logged = false;
        self.seen.clear();
        self.names.clear();
    }
}
