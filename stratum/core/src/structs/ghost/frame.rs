use std::sync::Arc;

use super::{GhostDraw, GhostJob, GhostPalette};
use crate::structs::marker::Rotation;

// Everything the shim needs to render one frame of the ghost. Core refills one value in place.
//
// The shim applies a frame in this order: `clear_all`, then `frees`, then `jobs`, then `draws`.
// A slot can be in both `frees` and `jobs` of one frame; it is freed first and then built.
#[derive(Debug, Clone)]
pub struct GhostFrame {
    // The build revision the jobs and draws belong to. The shim drops a job of another generation.
    pub generation: u64,
    // Moves each time core sees the world change or the player leave it. The shim drops its meshes and block
    // lookups when it changes, because they belong to the world that was left.
    pub world_epoch: u64,
    // The player is in a world as of this tick. Between two worlds the epoch moves while this is false.
    pub in_world: bool,
    // Free every slot before anything else in this frame.
    pub clear_all: bool,
    // Size of the shim's slot table.
    pub slot_capacity: u32,
    pub draws: Vec<GhostDraw>,
    pub jobs: Vec<GhostJob>,
    pub frees: Vec<u32>,
    // Set only in the frame where the generation changed.
    pub palette: Option<Arc<GhostPalette>>,
    pub opacity: f32,
    // The turn of the build the jobs belong to. The shim turns each block state by it.
    pub rotation: Rotation,
    // A wanted section got no slot, or took the slot of a farther one.
    pub cut: bool,
}

impl GhostFrame {
    pub fn new() -> Self {
        Self {
            generation: 0,
            world_epoch: 0,
            in_world: false,
            clear_all: false,
            slot_capacity: 0,
            draws: Vec::new(),
            jobs: Vec::new(),
            frees: Vec::new(),
            palette: None,
            opacity: 1.0,
            rotation: Rotation::R0,
            cut: false,
        }
    }
}

impl Default for GhostFrame {
    fn default() -> Self {
        Self::new()
    }
}
