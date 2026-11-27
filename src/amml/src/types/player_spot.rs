// Where the local player stands: the dimension id as the game holds it, and the block of the feet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerSpot {
    pub dimension: i32,
    pub feet: [i32; 3],
}

impl PlayerSpot {
    /// A feet level a little under a whole block still counts as that block: the game's collision box of a player
    /// standing on a block top can sit a few millionths under it. No block top is this close under a whole block.
    pub const FEET_TOLERANCE: f32 = 0.01;

    /// The block of feet at `position`, the lowest point of the player's box; x and z round down, also below zero.
    pub fn feet_of(position: [f32; 3]) -> [i32; 3] {
        [position[0].floor() as i32, (position[1] + Self::FEET_TOLERANCE).floor() as i32, position[2].floor() as i32]
    }
}
