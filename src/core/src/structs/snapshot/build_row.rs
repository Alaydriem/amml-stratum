use crate::structs::build::{BuildNotice, BuildPhase, BuildSettings};
use crate::structs::library::StructureId;

// What the settings panel and the HUD build card show. Built by the tick, never by a frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildRow {
    pub structure: StructureId,
    pub structure_name: String,
    pub phase: BuildPhase,
    pub settings: BuildSettings,
    // The layer being placed, counted from 1; 0 before the first block.
    pub layer: u32,
    pub layers: u32,
    pub placed: u64,
    pub total: u64,
    // Whole seconds, rounded up: the blocks left divided by the rate.
    pub eta_seconds: u64,
    // The names of the blocks the game does not know, each once. At most one per palette entry.
    pub unplaceable: Vec<String>,
    // The message of the last Start that did not run.
    pub notice: Option<BuildNotice>,
    // The first Down press came and a second one confirms the stop.
    pub confirm_stop: bool,
}
