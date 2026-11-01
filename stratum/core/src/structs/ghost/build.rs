use super::{GhostPalette, GhostPiece};

// The active marker's blocks in world space, with one palette shared by every piece.
#[derive(Debug, Clone)]
pub struct GhostBuild {
    pub pieces: Vec<GhostPiece>,
    pub palette: GhostPalette,
    // The palette id of the structure block drawn at the build's lowest corner.
    pub marker_block: u16,
    pub world_min: [i32; 3],
    // Inclusive.
    pub world_max: [i32; 3],
    // The build's generation, stored as given.
    pub revision: u64,
}

impl GhostBuild {
    // The build palette id of the block at a world cell. The first piece whose box holds the cell
    // decides, even when its cell is empty. None for no block, air, structure void, or a cell
    // outside every piece.
    pub fn block_at(&self, world: [i32; 3]) -> Option<u16> {
        for piece in &self.pieces {
            let size = piece.structure.size;
            let mut local = [0_u32; 3];
            let mut inside = true;
            for (axis, limit) in [size.x, size.y, size.z].into_iter().enumerate() {
                let delta = i64::from(world[axis]) - i64::from(piece.world_min[axis]);
                match u32::try_from(delta) {
                    Ok(value) if value < limit => local[axis] = value,
                    _ => {
                        inside = false;
                        break;
                    }
                }
            }
            if !inside {
                continue;
            }
            let index = piece.structure.index_of(local[0], local[1], local[2]);
            let palette_index = piece.structure.primary.get(index)?;
            let id = piece.remap.get(usize::from(palette_index)).copied()?;
            return Some(id).filter(|id| *id != GhostPalette::NONE);
        }
        None
    }
}
