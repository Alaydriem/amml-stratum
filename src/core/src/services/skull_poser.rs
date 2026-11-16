use crate::services::NbtReader;
use crate::structs::ghost::SkullPose;
use crate::structs::marker::Rotation;
use crate::structs::nbt::{NbtLimits, NbtTag};
use crate::structs::structure::{BlockState, StateValue};

// Works out the face and the turn the game draws a skull ghost with.
pub struct SkullPoser;

impl SkullPoser {
    // `facing_direction` of a wall skull in clockwise order seen from above: north, east, south, west. Down (0)
    // and up (1) do not turn.
    const WALL_RING: [u8; 4] = [2, 5, 3, 4];

    // The face of a skull standing on the floor.
    const FLOOR: u8 = 1;

    // Degrees of a `Rot` step.
    const ROT_STEP: f32 = 22.5;

    // The pose of the skull block `block` with the cell's NBT `nbt`, in a ghost turned by `rotation`. The
    // block state keeps its source direction, so the turn is applied here.
    pub fn pose(block: &BlockState, nbt: Option<&[u8]>, rotation: Rotation) -> SkullPose {
        let face = match block.states.get("facing_direction") {
            Some(StateValue::Int(value)) => u8::try_from(*value).unwrap_or(Self::FLOOR),
            _ => Self::FLOOR,
        };
        let face = match Self::WALL_RING.iter().position(|ring| *ring == face) {
            Some(at) => Self::WALL_RING[(at + usize::from(rotation.quarters())) % 4],
            None => face,
        };
        // The renderer turns only a skull on the floor; a wall face sets its own turn.
        let degrees = if face == Self::FLOOR {
            (Self::source_degrees(nbt) + f32::from(rotation.degrees())).rem_euclid(360.0)
        } else {
            0.0
        };
        SkullPose::new(face, degrees)
    }

    // The turn the NBT holds: `Rot` in steps of 22.5 degrees, else `Rotation` in degrees, else 0. This is the
    // order `SkullBlockActor::load` reads them in. A `Rotation` that is NaN or infinite is 0, so the game never
    // builds a matrix from it.
    fn source_degrees(nbt: Option<&[u8]>) -> f32 {
        let Some((_, root)) = nbt.and_then(|bytes| NbtReader::read(bytes, NbtLimits::STRUCTURE).ok()) else {
            return 0.0;
        };
        let degrees = match (root.get("Rot"), root.get("Rotation")) {
            (Some(NbtTag::Byte(step)), _) => f32::from(*step) * Self::ROT_STEP,
            (_, Some(NbtTag::Float(degrees))) => *degrees,
            (_, Some(NbtTag::Double(degrees))) => *degrees as f32,
            _ => 0.0,
        };
        if degrees.is_finite() { degrees } else { 0.0 }
    }
}
