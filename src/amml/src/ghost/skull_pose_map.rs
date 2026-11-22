use amml_api::{Face, SkullPose};
use stratum_core::structs::ghost::SkullPose as CoreSkullPose;

/// Turns core's skull pose (the face as the game's facing number, the turn in degrees) into the loader's (a face and
/// the turn in sixteenths).
pub struct SkullPoseMap;

impl SkullPoseMap {
    const DEGREES_PER_STEP: f32 = 22.5;
    const STEPS: f32 = 16.0;

    /// None for a facing number that is not a face the skull stands on or hangs from.
    pub fn pose(pose: CoreSkullPose) -> Option<SkullPose> {
        let face = match pose.face {
            1 => Face::Up,
            2 => Face::North,
            3 => Face::South,
            4 => Face::West,
            5 => Face::East,
            _ => return None,
        };
        let steps = (pose.degrees / Self::DEGREES_PER_STEP).round().rem_euclid(Self::STEPS);
        let turn = if face == Face::Up { steps as u8 } else { 0 };
        Some(SkullPose { face, turn })
    }
}
