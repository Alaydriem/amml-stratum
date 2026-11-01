use super::{GhostJobMode, SectionKey};

// One section mesh to build: the part's cells plus one cell of neighbours on every side.
#[derive(Debug, Clone)]
pub struct GhostJob {
    pub key: SectionKey,
    // The mesh slot the job builds into. Set by the scheduler.
    pub slot: u32,
    // Names this job to the report that answers it. Set by the scheduler.
    pub serial: u32,
    // World position of the part's minimum cell, padding excluded.
    pub origin: [i32; 3],
    // Size of the part in cells, padding excluded.
    pub dims: [u32; 3],
    // Padded grid of `dims + 2` per axis, X fastest, then Z, then Y, in build palette ids with
    // `u16::MAX` for empty.
    pub cells: Vec<u16>,
    // Leave out a cell whose world block has the same name as its ghost block.
    pub skip_matching: bool,
    // How the mesh lands in the slot.
    pub mode: GhostJobMode,
    // Names the group of jobs that build one slot mesh. Zero for a Replace job.
    pub group_serial: u32,
    // Position of the job in its group. Zero for a Replace job.
    pub group_index: u32,
    // Offset of the job's mesh inside the slot box, in cells. Zero for a Replace job.
    pub group_offset: [i32; 3],
    // Size of the slot box in cells. Equal to `dims` for a Replace job.
    pub slot_size: [u32; 3],
}

impl GhostJob {
    pub fn new(key: SectionKey) -> Self {
        Self {
            key,
            slot: 0,
            serial: 0,
            origin: [0; 3],
            dims: [0; 3],
            cells: Vec::new(),
            skip_matching: true,
            mode: GhostJobMode::Replace,
            group_serial: 0,
            group_index: 0,
            group_offset: [0; 3],
            slot_size: [0; 3],
        }
    }
}
