use stratum_core::structs::build::ClearPart;

// Measures clear parts.
pub struct Volume;

impl Volume {
    // Blocks in the box of a part, both corners included.
    pub fn of(part: &ClearPart) -> u64 {
        (0..3).map(|axis| (i64::from(part.max[axis]) - i64::from(part.min[axis]) + 1) as u64).product()
    }
}
