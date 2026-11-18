// The worker's answer to a `CountRequest`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CountResult {
    pub serial: u64,
    pub total: u64,
}
