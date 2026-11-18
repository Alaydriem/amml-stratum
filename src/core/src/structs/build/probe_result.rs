use super::PaletteOutcome;

// The game's answer to a `BuildProbeRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildProbeResult {
    // One outcome per palette entry, in palette order.
    pub outcomes: Vec<PaletteOutcome>,
    // Palette entries where the game's own turn of the block differs from the turn core made.
    pub turn_mismatches: Vec<u16>,
}
