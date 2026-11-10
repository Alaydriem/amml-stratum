use crate::structs::block::Face;

// The flat light the preview gives a block side.
pub struct PreviewShade;

impl PreviewShade {
    // The light the game gives `face`, out of 255: the top full, the bottom half, the sides between.
    pub fn of(face: Face) -> u8 {
        match face {
            Face::Up => 255,
            Face::Down => 128,
            Face::North | Face::South => 204,
            Face::West | Face::East => 153,
        }
    }
}
