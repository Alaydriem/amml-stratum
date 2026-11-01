use std::ffi::c_void;

use amml_game::BlockPos;
use stratum_core::structs::easy_place::SpotWorld;

use crate::game::WorldBlocks;

/// Reads the six neighbours of the spot the core aimed at: a neighbour can be placed against when it cannot be built
/// over and is not an interactive block (a door, a chest or a sign would open instead of taking the block).
/// Read-only.
pub struct SpotReader;

impl SpotReader {
    // the step to the neighbour on each face, in the order `SpotWorld::against` uses: Down, Up, North, South, West,
    // East
    const NORMALS: [[i32; 3]; 6] = [[0, -1, 0], [0, 1, 0], [0, 0, -1], [0, 0, 1], [-1, 0, 0], [1, 0, 0]];

    /// What surrounds `spot`.
    ///
    /// # Safety
    /// `region` must be the live `BlockSource` of the world.
    #[allow(unsafe_code)]
    pub unsafe fn read(blocks: &WorldBlocks, region: *mut c_void, spot: [i32; 3]) -> SpotWorld {
        let mut world = SpotWorld { against: [false; 6] };
        for (face, normal) in Self::NORMALS.iter().enumerate() {
            let pos = BlockPos { x: spot[0] + normal[0], y: spot[1] + normal[1], z: spot[2] + normal[2] };
            // SAFETY: forwarded to the caller
            world.against[face] = unsafe { blocks.against(region, &pos) };
        }
        world
    }
}
