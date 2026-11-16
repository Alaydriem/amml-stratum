use std::sync::{Arc, OnceLock};

use super::{ChestPairNbt, GhostBlockEntityTable, GhostPalette, GhostPiece};
use crate::structs::marker::Rotation;

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
    // The turn the pieces carry. Block states keep their source direction; this is the turn they need.
    pub rotation: Rotation,
    block_entities: OnceLock<Arc<GhostBlockEntityTable>>,
}

impl GhostBuild {
    // A build of `pieces`. The block entity table is built on first use.
    pub fn new(
        pieces: Vec<GhostPiece>,
        palette: GhostPalette,
        marker_block: u16,
        world_min: [i32; 3],
        world_max: [i32; 3],
        revision: u64,
        rotation: Rotation,
    ) -> Self {
        Self {
            pieces,
            palette,
            marker_block,
            world_min,
            world_max,
            revision,
            rotation,
            block_entities: OnceLock::new(),
        }
    }

    // This build with the block entity table of `previous` when both hold the same structures, in the same order,
    // at the same turn: the table is then the same `Arc` and is not built again.
    pub fn sharing_table_with(mut self, previous: &GhostBuild) -> Self {
        let same = previous.rotation == self.rotation
            && previous.pieces.len() == self.pieces.len()
            && previous.pieces.iter().zip(&self.pieces).all(|(old, new)| Arc::ptr_eq(&old.structure, &new.structure));
        if same {
            self.block_entities = OnceLock::from(previous.shared_block_entities());
        }
        self
    }

    // The NBT the game gets for the build's block entity cells.
    pub fn block_entities(&self) -> &GhostBlockEntityTable {
        self.block_entities.get_or_init(|| Arc::new(GhostBlockEntityTable::collect(&self.pieces)))
    }

    // The table, shared with the frames that carry it.
    pub fn shared_block_entities(&self) -> Arc<GhostBlockEntityTable> {
        self.block_entities();
        self.block_entities.get().map(Arc::clone).unwrap_or_default()
    }

    // The piece that holds a world cell and the cell's position inside its structure, by the same rule
    // as `block_at`.
    pub fn locate(&self, world: [i32; 3]) -> Option<(usize, [u32; 3])> {
        for (at, piece) in self.pieces.iter().enumerate() {
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
            if inside {
                return Some((at, local));
            }
        }
        None
    }

    // The id in `GhostBlockEntityTable::from_build` of the NBT at `local` of piece `piece`, or None for
    // a cell with no NBT or one past the table bound.
    pub fn block_entity_id(&self, piece: usize, local: [u32; 3]) -> Option<u32> {
        self.block_entities().id_of(self.entry_index(piece, local)?)
    }

    // The double chest keys the NBT at `local` of piece `piece` holds, read from the table with no NBT read.
    // None for a cell with no NBT, no pair keys, or one past the table bound.
    pub fn chest_pair(&self, piece: usize, local: [u32; 3]) -> Option<ChestPairNbt> {
        self.block_entities().chest_pair(self.entry_index(piece, local)?)
    }

    // The place of the NBT at `local` of piece `piece` among every piece's entries end to end.
    fn entry_index(&self, piece: usize, local: [u32; 3]) -> Option<usize> {
        let found = self.pieces.get(piece)?;
        let entry =
            found.structure.block_entities.entry_of(found.structure.index_of(local[0], local[1], local[2]) as u32)?;
        let before: usize = self.pieces[..piece].iter().map(|p| p.structure.block_entities.entry_count()).sum();
        Some(before + entry as usize)
    }

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
