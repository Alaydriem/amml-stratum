// Where the local player stands: the dimension id as the game holds it, and the block of the feet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerSpot {
    pub dimension: i32,
    pub feet: [i32; 3],
}

impl PlayerSpot {
    /// A player's eye above its feet.
    pub const EYE_HEIGHT: f32 = 1.62;
    /// The game's eye height is not exact in f32: seen in game, a player standing on y 72 has its eye at 73.6198, a
    /// little under 72 + EYE_HEIGHT. No block top is this close under a whole block.
    pub const FEET_TOLERANCE: f32 = 0.01;

    /// The block of the feet of a player whose eye is at `eye`; x and z round down, also below zero.
    pub fn feet_of(eye: [f32; 3]) -> [i32; 3] {
        [
            eye[0].floor() as i32,
            (eye[1] - Self::EYE_HEIGHT + Self::FEET_TOLERANCE).floor() as i32,
            eye[2].floor() as i32,
        ]
    }
}
