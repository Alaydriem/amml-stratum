use super::GhostSections;
use crate::structs::ghost::{GhostBuild, SectionKey};

// A scan of a build's sections that runs a few sections per call, so a big build never holds the
// game thread for a whole scan. The cursor walks Y, then X, then Z, as `GhostSections::occupied`.
pub struct SectionScan {
    low: [i32; 3],
    high: [i32; 3],
    cursor: [i32; 3],
    done: bool,
}

impl SectionScan {
    pub fn new(build: &GhostBuild) -> Self {
        let (low, high) = GhostSections::section_bounds(build);
        Self { low, high, cursor: [low[0], low[1], low[2]], done: false }
    }

    // Reads at most `budget` sections that a piece reaches into and pushes their keys to `out` and
    // their filled cell counts to `cells`, in the same order.
    // Sections no piece reaches are skipped without reading a cell and cost nothing. Returns true
    // once the whole box is scanned.
    pub fn step(
        &mut self,
        build: &GhostBuild,
        cells_per_frame: u32,
        budget: usize,
        out: &mut Vec<SectionKey>,
        cells: &mut Vec<u32>,
    ) -> bool {
        let mut left = budget;
        while !self.done && left > 0 {
            let [sx, sy, sz] = self.cursor;
            if GhostSections::push_whole(build, [sx, sy, sz], cells_per_frame, out, cells) {
                left -= 1;
            }
            self.advance();
        }
        self.done
    }

    pub fn done(&self) -> bool {
        self.done
    }

    fn advance(&mut self) {
        if self.cursor[2] < self.high[2] {
            self.cursor[2] += 1;
        } else if self.cursor[0] < self.high[0] {
            self.cursor[2] = self.low[2];
            self.cursor[0] += 1;
        } else if self.cursor[1] < self.high[1] {
            self.cursor[2] = self.low[2];
            self.cursor[0] = self.low[0];
            self.cursor[1] += 1;
        } else {
            self.done = true;
        }
    }
}
