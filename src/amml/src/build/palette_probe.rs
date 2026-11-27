use stratum_core::structs::build::{BuildProbeRequest, BuildProbeResult, PaletteOutcome};

use crate::traits::PaletteGame;
use crate::types::PaletteLookup;

/// Answers the core's palette request from the game: each entry is looked up as the ghost looks it up (exact, then the
/// name with the default states, then unknown), and the game's own turn of the block is compared with the block of
/// the entry core turned. Blocks are compared by handle: an equal handle is the same game block.
pub struct PaletteProbe;

impl PaletteProbe {
    pub fn answer(game: &dyn PaletteGame, request: &BuildProbeRequest) -> BuildProbeResult {
        let mut outcomes = Vec::with_capacity(request.entries.len());
        let mut turn_mismatches = Vec::new();
        for (index, (unturned, turned)) in request.entries.iter().enumerate() {
            let (outcome, mismatch) = Self::entry(game, unturned, turned, request.quarters);
            outcomes.push(outcome);
            if mismatch && let Ok(index) = u16::try_from(index) {
                turn_mismatches.push(index);
            }
        }
        BuildProbeResult { outcomes, turn_mismatches }
    }

    // the outcome of one entry, and whether the game turns its block differently from core
    fn entry(game: &dyn PaletteGame, unturned: &[u8], turned: &[u8], quarters: u8) -> (PaletteOutcome, bool) {
        if unturned.is_empty() || turned.is_empty() {
            return (PaletteOutcome::Unknown, false);
        }
        let found = game.lookup(unturned);
        let (outcome, block) = match found {
            PaletteLookup::Exact(block) => (PaletteOutcome::Exact, block),
            PaletteLookup::Default(block) => (PaletteOutcome::NameOnly, block),
            PaletteLookup::Unknown => return (PaletteOutcome::Unknown, false),
        };
        let game_turn = game.turn(block, quarters);
        let core_turn = game.lookup(turned).block();
        (outcome, game_turn.is_none() || game_turn != core_turn)
    }
}
