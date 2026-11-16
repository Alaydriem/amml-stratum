// How the game draws a skull ghost: the face it stands on and its turn.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SkullPose {
    // 1 for a skull on the floor, 2 to 5 for a skull on a wall, as `renderSkull` reads it.
    pub face: u8,
    // Clockwise turn in degrees, 0 up to 360. Always 0 for a wall face, which the renderer does not turn.
    pub degrees: f32,
}

impl SkullPose {
    pub fn new(face: u8, degrees: f32) -> Self {
        Self { face, degrees }
    }
}
