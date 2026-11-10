// A placement that waits for the server to confirm it.
#[derive(Debug, Clone)]
pub(crate) struct PendingPlace {
    pub spot: [i32; 3],
    // The block name that counts as confirmed.
    pub expected: String,
    // The last block name seen at the spot. Empty when none was seen.
    pub seen: String,
    // For a double slab, the single slab block name that also counts as confirmed. Empty for any
    // other block.
    pub slab: String,
    // True when this placement is the first half of a double slab.
    pub first_half: bool,
    pub deadline: f64,
}

impl PendingPlace {
    const NAME_CAPACITY: usize = 48;

    pub fn new() -> Self {
        Self {
            spot: [0; 3],
            expected: String::with_capacity(Self::NAME_CAPACITY),
            seen: String::with_capacity(Self::NAME_CAPACITY),
            slab: String::with_capacity(Self::NAME_CAPACITY),
            first_half: false,
            deadline: 0.0,
        }
    }
}

impl Default for PendingPlace {
    fn default() -> Self {
        Self::new()
    }
}
