// The palette of a build, for the game to look up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildProbeRequest {
    // Per palette entry, in palette order: the NBT of the block as the structure holds it, and the NBT of the
    // block core turned by the marker rotation. An empty buffer is an entry that could not be written; it
    // resolves to `Unknown`.
    pub entries: Vec<(Vec<u8>, Vec<u8>)>,
    // The marker rotation in clockwise quarter turns, 0 to 3, so the game can turn the first block of each pair
    // itself and compare.
    pub quarters: u8,
}
