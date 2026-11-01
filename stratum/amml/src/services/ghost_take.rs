use stratum_core::structs::ghost::{GhostDraw, GhostJob};

use super::GhostOutbox;

// The render thread's copy of the last ghost frame it took. Every buffer is reused, so a take allocates only while a
// buffer grows; a repeated take keeps the jobs it does not hand out, so the next receive gives their cell buffers
// back to the outbox.
//
// Bound: `draws`, `jobs` and `frees` hold one frame's worth, at most `slot_capacity` entries each; the palette is
// bounded by core's palette limit.
pub struct GhostTake {
    generation: u64,
    world_epoch: u64,
    in_world: bool,
    clear_all: bool,
    slot_capacity: u32,
    opacity: f32,
    // Clockwise quarter turns, 0 to 3.
    rotation: u8,
    draws: Vec<GhostDraw>,
    jobs: Vec<(u64, GhostJob)>,
    // The jobs came with this take; false after a repeat, which hands out none.
    fresh_jobs: bool,
    frees: Vec<u32>,
    has_palette: bool,
    palette_bytes: Vec<u8>,
    palette_ends: Vec<usize>,
}

impl GhostTake {
    pub fn new() -> Self {
        Self {
            generation: 0,
            world_epoch: 0,
            in_world: false,
            clear_all: false,
            slot_capacity: 0,
            opacity: 1.0,
            rotation: 0,
            draws: Vec::new(),
            jobs: Vec::new(),
            fresh_jobs: false,
            frees: Vec::new(),
            has_palette: false,
            palette_bytes: Vec::new(),
            palette_ends: Vec::new(),
        }
    }

    // Takes everything the outbox holds.
    pub fn receive(&mut self, outbox: &mut GhostOutbox) {
        self.generation = outbox.generation();
        self.world_epoch = outbox.world_epoch();
        self.in_world = outbox.in_world();
        self.clear_all = outbox.clear_all();
        self.slot_capacity = outbox.slot_capacity();
        self.opacity = outbox.opacity();
        self.rotation = outbox.rotation().quarters();
        self.draws.clear();
        self.draws.extend_from_slice(outbox.draws());
        self.has_palette =
            outbox.hand_over(&mut self.jobs, &mut self.frees, &mut self.palette_bytes, &mut self.palette_ends);
        self.fresh_jobs = true;
    }

    // Nothing new since the last take: the same draws, and no clear, frees, jobs or palette.
    pub fn repeat(&mut self) {
        self.clear_all = false;
        self.fresh_jobs = false;
        self.frees.clear();
        self.has_palette = false;
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

    // 0 to 1, the alpha of every ghost vertex.
    pub fn opacity(&self) -> f32 {
        self.opacity
    }

    // 0 to 3 clockwise quarter turns seen from above, as Minecraft's Rotate90: each block state is turned by it.
    pub fn rotation(&self) -> u8 {
        self.rotation
    }

    // Each job with the generation of the frame that gave it.
    pub fn jobs(&self) -> &[(u64, GhostJob)] {
        if self.fresh_jobs { &self.jobs } else { &[] }
    }

    pub fn frees(&self) -> &[u32] {
        &self.frees
    }

    // The NBT bytes of each palette entry when the palette changed in this take.
    pub fn palette(&self) -> Option<impl Iterator<Item = &[u8]>> {
        self.has_palette.then(|| {
            let mut start = 0;
            self.palette_ends.iter().map(move |end| {
                let entry = &self.palette_bytes[start..*end];
                start = *end;
                entry
            })
        })
    }
}

impl Default for GhostTake {
    fn default() -> Self {
        Self::new()
    }
}
