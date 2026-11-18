use super::{BuildPass, BuildStep};

// The order of one layer, as the worker worked it out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderResult {
    pub serial: u64,
    pub pass: BuildPass,
    pub layer: u32,
    pub steps: Vec<BuildStep>,
}
