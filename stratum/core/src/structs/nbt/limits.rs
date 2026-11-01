#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NbtLimits {
    // Deepest nesting of compounds and lists. The root compound is depth 1.
    pub max_depth: u16,
    // Most items one list or array may claim.
    pub max_list_len: u32,
    // Most bytes the decoded tree may use.
    pub max_total_bytes: u64,
    // Longest list of Int or Int Array decoded to `IntList` / `IntArray`. A longer one becomes
    // `IntListSpan` / `IntArraySpan`, charged at 2 bytes per item: the size of the compact layer
    // its reader builds.
    pub max_decoded_int_list: u32,
}

impl NbtLimits {
    // Sized for a 256 x 384 x 256 structure: one list item per cell, and room for both index
    // lists as spans at 2 bytes per item (96 MiB) plus 64 MiB for the palette and the rest.
    pub const STRUCTURE: NbtLimits = NbtLimits {
        max_depth: 64,
        max_list_len: 25_165_824,
        max_total_bytes: 2 * 25_165_824 * 2 + 64 * 1024 * 1024,
        max_decoded_int_list: 4096,
    };
}
