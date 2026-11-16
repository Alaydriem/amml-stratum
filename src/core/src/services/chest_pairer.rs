use crate::services::BlockEntityNames;
use crate::structs::ghost::{ChestPair, ChestRole, GhostBuild};
use crate::structs::marker::Rotation;
use crate::structs::structure::{BlockState, StateValue};

// Decides which cells of a ghost build are the two halves of a double chest.
pub struct ChestPairer;

impl ChestPairer {
    // `minecraft:cardinal_direction` values in clockwise order seen from above.
    const CARDINAL_RING: [&'static str; 4] = ["north", "east", "south", "west"];

    // `facing_direction` values in the same order.
    const FACING_RING: [i32; 4] = [2, 5, 3, 4];

    // The role of the chest at world cell `cell`. Two chests are a double chest when each names the
    // other half in its NBT, both are the same block, and the other half is one block away on X or Z, beside
    // the chest's facing and not in front of or behind it. The NBT of a turned build is turned with it, so the
    // direction is in the turned frame, and the facing is turned to match. A cell that is not a chest, or
    // whose other half is missing, is single.
    pub fn pair(build: &GhostBuild, cell: [i32; 3]) -> ChestRole {
        Self::decide(build, cell).unwrap_or(ChestRole::Single)
    }

    fn decide(build: &GhostBuild, cell: [i32; 3]) -> Option<ChestRole> {
        let block = Self::block(build, cell)?;
        let (offset, own_lead) = Self::pair_data(build, cell)?;
        let direction = ChestPair::from_offset(offset)?;
        if !Self::beside(block, build.rotation, direction) {
            return None;
        }
        let other = [cell[0].checked_add(offset[0])?, cell[1].checked_add(offset[1])?, cell[2].checked_add(offset[2])?];
        if Self::block(build, other)? != block {
            return None;
        }
        let (back, other_lead) = Self::pair_data(build, other)?;
        if back != offset.map(|value| -value) {
            return None;
        }
        let lead = match (own_lead, other_lead) {
            (true, false) => true,
            (false, true) => false,
            // Both halves or none say they lead: the half with the smaller coordinate on the pair axis.
            _ => offset[0] > 0 || offset[2] > 0,
        };
        Some(if lead { ChestRole::Lead(direction) } else { ChestRole::Follower })
    }

    // True when `direction` runs beside the facing of `block` turned by `rotation`: along X for a chest that
    // faces north or south, along Z for one that faces east or west. A chest with no facing state passes.
    fn beside(block: &BlockState, rotation: Rotation, direction: ChestPair) -> bool {
        let source = match (block.states.get("minecraft:cardinal_direction"), block.states.get("facing_direction")) {
            (Some(StateValue::Text(facing)), _) => Self::CARDINAL_RING.iter().position(|ring| ring == facing),
            (_, Some(StateValue::Int(facing))) => Self::FACING_RING.iter().position(|ring| ring == facing),
            _ => None,
        };
        let Some(source) = source else { return true };
        let faces_along_z = (source + usize::from(rotation.quarters())) % 2 == 0;
        let along_x = matches!(direction, ChestPair::PlusX | ChestPair::MinusX);
        faces_along_z == along_x
    }

    // The block name and state of the chest at `cell`, or None for any other cell.
    fn block(build: &GhostBuild, cell: [i32; 3]) -> Option<&BlockState> {
        let id = build.block_at(cell)?;
        let block = build.palette.entries.get(usize::from(id))?;
        BlockEntityNames::is_chest(&block.name).then_some(block)
    }

    // The offset to the other half named by the NBT of the cell, and whether the cell says it leads. The pair
    // keys come from the build's block entity table, which read them once.
    fn pair_data(build: &GhostBuild, cell: [i32; 3]) -> Option<([i32; 3], bool)> {
        let (piece, local) = build.locate(cell)?;
        let pair = build.chest_pair(piece, local)?;
        let offset = pair.offset(local.map(|value| value as i32), build.pieces[piece].structure.world_origin)?;
        Some((offset, pair.lead))
    }
}
