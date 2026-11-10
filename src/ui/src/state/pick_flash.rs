use stratum_core::structs::library::StructureId;
use stratum_core::structs::snapshot::ImportPick;

use super::Page;

// Follows the latest file picker answer: which page to move to, which rows flash and which row to scroll to. It
// holds the ids of one pick only; the next serial replaces them.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PickFlash {
    serial: Option<u64>,
    moved: bool,
    // The ids of the pick being followed and the time each was first seen. Bounded by one pick; replaced by the next.
    seen: Vec<(StructureId, f64)>,
    scroll: Option<StructureId>,
}

impl PickFlash {
    // Seconds a new row takes to fade from full flash to none.
    pub const FLASH_SECONDS: f64 = 2.0;

    pub fn new() -> Self {
        Self::default()
    }

    // The follower for the next open of the library: it keeps the picks it has seen and drops a waiting scroll.
    pub fn after_close(mut self) -> Self {
        self.scroll = None;
        self
    }

    // Takes the latest pick. Returns the page to move to once per pick, when it has at least one structure.
    pub fn observe(&mut self, pick: Option<&ImportPick>, now: f64) -> Option<Page> {
        let pick = pick?;
        if self.serial != Some(pick.serial) {
            self.serial = Some(pick.serial);
            self.moved = false;
            self.seen.clear();
            self.scroll = None;
        }
        for id in &pick.structures {
            if self.seen.iter().any(|(seen, _)| seen == id) {
                continue;
            }
            if self.seen.is_empty() {
                self.scroll = Some(*id);
            }
            self.seen.push((*id, now));
        }
        if self.moved || pick.structures.is_empty() {
            return None;
        }
        self.moved = true;
        Some(match pick.destination {
            Some(project) => Page::Project(project),
            None => Page::Unsorted,
        })
    }

    // 1.0 when the row was first seen, fading linearly to 0.0 after `FLASH_SECONDS`. Rows of no pick are 0.0.
    pub fn strength(&self, id: StructureId, now: f64) -> f32 {
        match self.seen.iter().find(|(seen, _)| *seen == id) {
            Some((_, first)) => (1.0 - (now - first) / Self::FLASH_SECONDS).clamp(0.0, 1.0) as f32,
            None => 0.0,
        }
    }

    // True once, for the first structure of the pick.
    pub fn take_scroll(&mut self, id: StructureId) -> bool {
        if self.scroll == Some(id) {
            self.scroll = None;
            return true;
        }
        false
    }

    // How many ids the store holds.
    pub fn tracked(&self) -> usize {
        self.seen.len()
    }
}
