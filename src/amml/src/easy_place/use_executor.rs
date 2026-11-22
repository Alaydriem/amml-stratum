use amml_api::{
    ApiError, BlockHandle, BlockPos, Face as ApiFace, Hud, InventoryMove, LocalPlayer, RequestId, RequestOptions, Vec3,
};
use stratum_core::structs::block::Face;
use stratum_core::structs::easy_place::{Placement, UseCommand};

use crate::types::UseOutcome;

/// Carries out the command the core gave, through the loader: selects a hotbar slot through the HUD, places a block
/// through the game mode, or asks for a swap or a pick. It decides nothing and holds no lock: `build_at` raises
/// `BuildBlock` and the block change on the same stack. A command it cannot carry out (a select the game did not take,
/// a build the game refused) places nothing, and a Place or Select and Place that placed nothing comes back as not
/// sent.
pub struct UseExecutor;

impl UseExecutor {
    const HOTBAR_SLOTS: u8 = 9;

    /// Carries out `command`; `pick` is the block of the palette id a Pick names, `counts` the stack counts of the
    /// inventory the command was decided on.
    pub fn run(
        player: &LocalPlayer<'_>,
        hud: &Hud<'_>,
        command: &UseCommand,
        pick: Option<BlockHandle>,
        counts: &[u8; 36],
    ) -> UseOutcome {
        let mut outcome = UseOutcome::default();
        match *command {
            UseCommand::Select { slot, place } => {
                if !Self::select(player, hud, slot) || !Self::place(player, &place) {
                    outcome.not_sent = Some(place.spot);
                }
            }
            UseCommand::Swap { from, to } => outcome.request = Some(Self::swap(player, counts, from, to)),
            UseCommand::Pick { slot, .. } => {
                let request = match (player.inventory(), pick) {
                    (Some(inventory), Some(block)) => inventory.request_pick(block, slot),
                    _ => Err(ApiError::Refused),
                };
                outcome.request = Some(request);
            }
            UseCommand::Place(place) => {
                if !Self::place(player, &place) {
                    outcome.not_sent = Some(place.spot);
                }
            }
            UseCommand::Vanilla | UseCommand::Block => {}
        }
        outcome
    }

    /// The API face of a core face.
    pub fn face(face: Face) -> ApiFace {
        match face {
            Face::Down => ApiFace::Down,
            Face::Up => ApiFace::Up,
            Face::North => ApiFace::North,
            Face::South => ApiFace::South,
            Face::West => ApiFace::West,
            Face::East => ApiFace::East,
        }
    }

    // into an empty hotbar slot the whole stack is placed; otherwise the two stacks swap
    fn swap(player: &LocalPlayer<'_>, counts: &[u8; 36], from: u8, to: u8) -> Result<RequestId, ApiError> {
        let inventory = player.inventory().ok_or(ApiError::Refused)?;
        let count = |slot: u8| counts.get(usize::from(slot)).copied().unwrap_or(0);
        if count(to) == 0 {
            let moves = [InventoryMove::Place { from, to, count: count(from) }];
            return inventory.request_moves(&moves, RequestOptions { hidden_open: true, predict: true });
        }
        inventory.request_swap(from, to)
    }

    // true when the slot is selected after the call
    fn select(player: &LocalPlayer<'_>, hud: &Hud<'_>, slot: u8) -> bool {
        if slot >= Self::HOTBAR_SLOTS {
            return false;
        }
        if let Err(error) = hud.select_hotbar_slot(slot) {
            curia::warn!("easy place select skipped", { slot: slot, error: format!("{error:?}") });
            return false;
        }
        let selected = player.inventory().and_then(|inventory| inventory.selected_slot());
        if selected == Some(slot) {
            return true;
        }
        curia::warn!("easy place select not taken", { slot: slot, selected: format!("{selected:?}") });
        false
    }

    // clicks the neighbour's face that points at the spot, at the point the core chose, the way the game's own use
    // does; true when the game built
    fn place(player: &LocalPlayer<'_>, place: &Placement) -> bool {
        let Some(game_mode) = player.game_mode() else { return false };
        let [x, y, z] = place.neighbour;
        let click = Vec3 { x: x as f32 + place.click[0], y: y as f32 + place.click[1], z: z as f32 + place.click[2] };
        match game_mode.build_at(BlockPos { x, y, z }, Self::face(place.face), click) {
            Ok(true) => {
                let [sx, sy, sz] = place.spot;
                curia::info!("easy place placed", { x: sx, y: sy, z: sz });
                true
            }
            Ok(false) => false,
            Err(error) => {
                curia::warn!("easy place build refused", { error: format!("{error:?}") });
                false
            }
        }
    }
}
