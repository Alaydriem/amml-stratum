// What the ghost reports about itself, for the debug overlay and the log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GhostTelemetry {
    // Current: mesh slots that hold or await a mesh.
    pub sections_live: u32,
    // Current.
    pub slot_capacity: u32,
    // The mesh budget from config, in bytes.
    pub budget_bytes: u64,
    // Current: reported bytes of the live slots, to read against `budget_bytes`.
    pub used_bytes: u64,
    // Current: heap bytes of the turned copies of the active build, charged to the structure cache
    // budget.
    pub turned_bytes: u64,
    // The structure cache budget from config, in bytes.
    pub cache_budget_bytes: u64,
    // Current: cached structures plus the charged turned copies, to read against `cache_budget_bytes`.
    pub cache_resident_bytes: u64,
    // Current: a wanted section got no slot in this generation.
    pub cut: bool,
    // Session total: jobs the shim reported as failed.
    pub failed_slots: u64,
    // Current: structures of the active build that could not be loaded.
    pub failed_loads: u32,
    // Session total: ticks in which more block changes arrived than the buffer holds.
    pub block_change_overflows: u64,
    // Current: blocks of the last resolved palette the game did not know, drawn as the unknown block.
    pub unknown_blocks: u64,
    // Current: blocks of the last resolved palette drawn with their default states because the stored states did
    // not match.
    pub default_state_blocks: u64,
    // Current: finished mini ghost groups, each one merged mesh.
    pub miniature_slots: u32,
    // Current: mini ghost sections built into finished groups.
    pub miniature_sections: u32,
    // Current: reported bytes of the finished mini ghost groups.
    pub miniature_bytes: u64,
    // Current: the mini ghost is on and the build has more sections than the mini ghost slot limit.
    pub miniature_over_limit: bool,
}
