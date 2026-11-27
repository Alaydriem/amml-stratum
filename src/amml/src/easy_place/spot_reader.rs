use amml_api::{BlockPos, BlockSource};
use stratum_core::structs::easy_place::SpotWorld;

/// Reads the six neighbours of the spot the core aimed at: a neighbour can be placed against when it cannot be built
/// over and is not an interactive block (a door, a chest or a sign would open instead of taking the block). A block the
/// loader cannot read is not placed against. Read-only.
pub struct SpotReader;

impl SpotReader {
    // the step to the neighbour on each face, in the order `SpotWorld::against` uses: Down, Up, North, South, West,
    // East
    const NORMALS: [[i32; 3]; 6] = [[0, -1, 0], [0, 1, 0], [0, 0, -1], [0, 0, 1], [-1, 0, 0], [1, 0, 0]];

    /// What surrounds `spot`.
    pub fn read(source: &BlockSource<'_>, spot: [i32; 3]) -> SpotWorld {
        let mut world = SpotWorld { against: [false; 6] };
        for (face, normal) in Self::NORMALS.iter().enumerate() {
            let pos = BlockPos { x: spot[0] + normal[0], y: spot[1] + normal[1], z: spot[2] + normal[2] };
            world.against[face] = Self::against(source, pos);
        }
        world
    }

    fn against(source: &BlockSource<'_>, pos: BlockPos) -> bool {
        let Some(block) = source.block(pos) else { return false };
        block.can_be_built_over() == Some(false) && block.is_interactive() == Some(false)
    }
}
