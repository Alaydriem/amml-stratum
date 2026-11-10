// Counters for the easy place service.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EasyPlaceTelemetry {
    pub placed: u64,
    pub confirmed: u64,
    pub rejected: u64,
    // Placements the shim could not carry out. They do not count as placed.
    pub not_sent: u64,
    pub wrong: u64,
    pub cancelled_missing: u64,
    pub cancelled_no_neighbour: u64,
    pub blocked_vanilla: u64,
}
