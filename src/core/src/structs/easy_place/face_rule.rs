// Which faces of a neighbour a block may be placed against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaceRule {
    Any,
    // Only against the up face of the block below.
    Floor,
    // Only against a side face of a horizontal neighbour.
    Wall,
}
