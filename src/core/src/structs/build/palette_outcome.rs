// How a block resolves against the game palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteOutcome {
    // The name and every state are known.
    Exact,
    // The name is known; the state list is dropped.
    NameOnly,
    // The name is not in the palette. The build skips these blocks.
    Unknown,
}
