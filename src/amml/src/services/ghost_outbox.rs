use stratum_core::structs::ghost::{GhostDraw, GhostFrame, GhostJob};
use stratum_core::structs::marker::Rotation;

// The ghost frames the tick published since the render thread last took one, folded into one. A frame that is not
// taken before the next publish latches: `clear_all` is OR'd, the newest palette is kept, and frees and jobs add
// up, except that a latched job is dropped when its slot is freed or given a new job later, or by any `clear_all`.
// Draws, opacity, rotation, generation and world epoch are always the newest.
//
// Bound: `jobs` and `frees` hold at most one entry per slot, so at most `slot_capacity` entries each; an entry for a
// slot past the capacity is left out. `latest` and `freed` hold one mark per slot, sized when the capacity changes.
// `spare` keeps at most `slot_capacity` jobs for their cell buffers. The palette is bounded by core's palette limit.
pub struct GhostOutbox {
    // A publish arrived that the render thread has not taken.
    pending: bool,
    generation: u64,
    world_epoch: u64,
    in_world: bool,
    clear_all: bool,
    slot_capacity: u32,
    draws: Vec<GhostDraw>,
    // Each job with the generation of the frame that gave it.
    jobs: Vec<(u64, GhostJob)>,
    frees: Vec<u32>,
    has_palette: bool,
    // The palette entries' NBT bytes end to end, and where each entry ends.
    palette_bytes: Vec<u8>,
    palette_ends: Vec<usize>,
    opacity: f32,
    rotation: Rotation,
    spare: Vec<GhostJob>,
    // Per slot: the serial of the latched job that still counts for it, so a later free or job drops an older one.
    latest: Vec<Option<u32>>,
    // Per slot: the slot is in `frees`.
    freed: Vec<bool>,
}

impl GhostOutbox {
    pub fn new() -> Self {
        Self {
            pending: false,
            generation: 0,
            world_epoch: 0,
            in_world: false,
            clear_all: false,
            slot_capacity: 0,
            draws: Vec::new(),
            jobs: Vec::new(),
            frees: Vec::new(),
            has_palette: false,
            palette_bytes: Vec::new(),
            palette_ends: Vec::new(),
            opacity: 1.0,
            rotation: Rotation::R0,
            spare: Vec::new(),
            latest: Vec::new(),
            freed: Vec::new(),
        }
    }

    pub fn is_pending(&self) -> bool {
        self.pending
    }

    // Folds the tick's frame into what waits for the render thread. Linear in the frame's entries.
    pub fn publish(&mut self, frame: &GhostFrame) {
        if frame.slot_capacity != self.slot_capacity || self.latest.len() != frame.slot_capacity as usize {
            self.slot_capacity = frame.slot_capacity;
            self.latest.clear();
            self.latest.resize(frame.slot_capacity as usize, None);
            self.freed.clear();
            self.freed.resize(frame.slot_capacity as usize, false);
            self.recycle_all();
            self.frees.clear();
        }
        if !self.pending {
            self.clear_all = false;
            self.has_palette = false;
            self.recycle_all();
            self.clear_frees();
        }
        self.pending = true;
        self.generation = frame.generation;
        self.world_epoch = frame.world_epoch;
        self.in_world = frame.in_world;
        self.opacity = frame.opacity;
        self.rotation = frame.rotation;
        self.draws.clear();
        self.draws.extend_from_slice(&frame.draws);
        if frame.clear_all {
            self.clear_all = true;
            self.recycle_all();
            self.clear_frees();
        }
        for slot in &frame.frees {
            let index = *slot as usize;
            if index >= self.freed.len() {
                continue;
            }
            self.latest[index] = None;
            if !std::mem::replace(&mut self.freed[index], true) {
                self.frees.push(*slot);
            }
        }
        for job in &frame.jobs {
            let index = job.slot as usize;
            if index >= self.latest.len() {
                continue;
            }
            let mut latched = self.spare.pop().unwrap_or_else(|| GhostJob::new(job.key));
            latched.clone_from(job);
            self.jobs.push((frame.generation, latched));
            self.latest[index] = Some(job.serial);
        }
        self.drop_superseded();
        if let Some(palette) = &frame.palette {
            self.write_palette(palette.entries.len(), |id, out| palette.entry_nbt(id, out).is_ok());
        }
    }

    // Moves what waits into the render side's buffers and takes theirs in exchange, so neither side allocates once
    // the buffers are grown. The caller's old jobs become spares.
    pub fn hand_over(
        &mut self,
        jobs: &mut Vec<(u64, GhostJob)>,
        frees: &mut Vec<u32>,
        palette_bytes: &mut Vec<u8>,
        palette_ends: &mut Vec<usize>,
    ) -> bool {
        for (_, job) in jobs.drain(..) {
            self.keep_spare(job);
        }
        for (_, job) in &self.jobs {
            self.latest[job.slot as usize] = None;
        }
        std::mem::swap(&mut self.jobs, jobs);
        for slot in &self.frees {
            self.freed[*slot as usize] = false;
        }
        frees.clear();
        std::mem::swap(&mut self.frees, frees);
        let has_palette = std::mem::take(&mut self.has_palette);
        if has_palette {
            std::mem::swap(&mut self.palette_bytes, palette_bytes);
            std::mem::swap(&mut self.palette_ends, palette_ends);
        }
        self.pending = false;
        has_palette
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn world_epoch(&self) -> u64 {
        self.world_epoch
    }

    pub fn in_world(&self) -> bool {
        self.in_world
    }

    pub fn clear_all(&self) -> bool {
        self.clear_all
    }

    pub fn slot_capacity(&self) -> u32 {
        self.slot_capacity
    }

    pub fn draws(&self) -> &[GhostDraw] {
        &self.draws
    }

    pub fn opacity(&self) -> f32 {
        self.opacity
    }

    pub fn rotation(&self) -> Rotation {
        self.rotation
    }

    // Writes `count` entries with `write`, which appends one entry's bytes and says whether it could. An entry that
    // could not be written is left empty and logged once per palette.
    fn write_palette(&mut self, count: usize, mut write: impl FnMut(u16, &mut Vec<u8>) -> bool) {
        self.palette_bytes.clear();
        self.palette_ends.clear();
        let mut failed = 0usize;
        for id in 0..count {
            let start = self.palette_bytes.len();
            let written = u16::try_from(id).is_ok_and(|id| write(id, &mut self.palette_bytes));
            if !written {
                self.palette_bytes.truncate(start);
                failed += 1;
            }
            self.palette_ends.push(self.palette_bytes.len());
        }
        if failed > 0 {
            curia::warn!("ghost palette entries not written, they draw as the unknown block", { failed: failed });
        }
        self.has_palette = true;
    }

    // Keeps, in order, only the jobs that still count for their slot; the rest become spares.
    fn drop_superseded(&mut self) {
        let mut kept = 0;
        for index in 0..self.jobs.len() {
            let (_, job) = &self.jobs[index];
            if self.latest[job.slot as usize] == Some(job.serial) {
                self.jobs.swap(kept, index);
                kept += 1;
            }
        }
        while self.jobs.len() > kept {
            if let Some((_, job)) = self.jobs.pop() {
                self.keep_spare(job);
            }
        }
    }

    fn recycle_all(&mut self) {
        while let Some((_, job)) = self.jobs.pop() {
            if let Some(mark) = self.latest.get_mut(job.slot as usize) {
                *mark = None;
            }
            self.keep_spare(job);
        }
    }

    fn clear_frees(&mut self) {
        for slot in self.frees.drain(..) {
            if let Some(mark) = self.freed.get_mut(slot as usize) {
                *mark = false;
            }
        }
    }

    fn keep_spare(&mut self, job: GhostJob) {
        if self.spare.len() < self.slot_capacity as usize {
            self.spare.push(job);
        }
    }
}

impl Default for GhostOutbox {
    fn default() -> Self {
        Self::new()
    }
}
