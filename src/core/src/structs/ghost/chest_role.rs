use super::ChestPair;

// What a chest cell is in a double chest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChestRole {
    // A chest with no other half. It draws as a small chest.
    Single,
    // The half that draws the large chest, toward the other half.
    Lead(ChestPair),
    // The other half. It draws nothing.
    Follower,
}
