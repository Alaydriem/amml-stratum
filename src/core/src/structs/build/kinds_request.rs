// A box of the world whose block kinds the game reads. Both corners are inclusive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildKindsRequest {
    pub min: [i32; 3],
    pub max: [i32; 3],
}
