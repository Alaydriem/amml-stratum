use std::ffi::c_void;

use amml_game::BlockPos;
use stratum_core::structs::easy_place::{Placement, UseCommand};

use crate::easy_place::{StackRequest, UseSignals};
use crate::game::{GameMode, PlayerInventory, PlayerView};
use crate::traits::UseCore;
use crate::types::{PlayerCalls, UseCalls};

// `void HudScreenController::_handleSlotSelection(int slot, ContainerID containerId)`: this, the slot, the container id
type HandleSlotSelectionFn = unsafe extern "C" fn(*mut c_void, i32, i32);

/// Carries out the command the core gave: selects a hotbar slot through the HUD, places a block through the game mode,
/// or hands a swap or a pick to the stack request. It decides nothing; a command it cannot carry out (no HUD to select
/// with, a select the game did not take, a build the game refused) places nothing, and a Place or Select and Place that
/// placed nothing is reported to the core as not sent. The HUD controller is held from its tick until it is destroyed.
pub struct UseExecutor {
    calls: UseCalls,
    players: PlayerCalls,
}

impl UseExecutor {
    const HOTBAR_SLOTS: u8 = 9;
    // a HUD controller not ticked for this long is not used
    const HUD_FRESH_SECONDS: f64 = 0.5;
    // ContainerID::Inventory
    const INVENTORY_CONTAINER: i32 = 0;
    // HitResultType::Tile
    const TILE: i32 = 0;

    pub fn new(calls: &UseCalls, players: &PlayerCalls) -> Self {
        Self { calls: *calls, players: *players }
    }

    /// Carries out `command`; `palette` gives the block of a palette id, null when there is none.
    ///
    /// # Safety
    /// `client` and `player` must be the live client and its live local player.
    #[allow(unsafe_code, clippy::too_many_arguments)]
    pub unsafe fn run(
        &self,
        signals: &UseSignals,
        request: &mut StackRequest,
        core: &mut dyn UseCore,
        now: f64,
        client: *mut c_void,
        player: *mut c_void,
        command: &UseCommand,
        palette: &mut dyn FnMut(u16) -> *const c_void,
    ) {
        match *command {
            UseCommand::Select { slot, place } => {
                // SAFETY: forwarded to the caller
                if !unsafe { self.select(signals, now, player, slot) }
                    || !unsafe { self.place(signals, player, &place) }
                {
                    core.not_sent(place.spot);
                }
            }
            // SAFETY: forwarded to the caller
            UseCommand::Swap { from, to } => unsafe {
                request.swap(now, client, player, i32::from(from), i32::from(to))
            },
            UseCommand::Pick { palette_id, slot } => {
                let block = palette(palette_id);
                // SAFETY: forwarded to the caller; the palette gives null or a block of the world's registry
                unsafe { request.pick(now, client, player, block, i32::from(slot)) };
            }
            UseCommand::Place(place) => {
                // SAFETY: forwarded to the caller
                if !unsafe { self.place(signals, player, &place) } {
                    core.not_sent(place.spot);
                }
            }
            UseCommand::Vanilla | UseCommand::Block => {}
        }
    }

    /// The HUD screen's tick: the controller the hotbar select goes through.
    pub fn capture_hud(signals: &UseSignals, hud: usize, now: f64) {
        signals.set_hud(hud, now);
        signals.clear_no_hud_logged();
    }

    /// A screen controller is being destroyed; when it is the captured HUD, the HUD is forgotten.
    pub fn forget_hud(signals: &UseSignals, controller: usize) {
        signals.forget_hud(controller);
    }

    /// Forgets the HUD and ends the own-build guard; on mod disable.
    pub fn reset(signals: &UseSignals) {
        signals.set_hud(0, 0.0);
        signals.set_own_build(false);
    }

    // true when the slot is selected after the call
    #[allow(unsafe_code)]
    unsafe fn select(&self, signals: &UseSignals, now: f64, player: *mut c_void, slot: u8) -> bool {
        // SAFETY: the caller guarantees a live player
        let Some(inventory) = (unsafe { PlayerInventory::of_player(player, &self.players) }) else { return false };
        if slot >= Self::HOTBAR_SLOTS {
            return false;
        }
        let (hud, seen) = signals.hud();
        let fresh = hud != 0 && now - seen < Self::HUD_FRESH_SECONDS;
        if !fresh || self.calls.handle_slot_selection == 0 {
            if !signals.take_no_hud_logged() {
                curia::warn!("easy place select skipped, no HUD", { slot: slot });
            }
            return false;
        }
        // SAFETY: the entry is the address of _handleSlotSelection
        let call: HandleSlotSelectionFn = unsafe { std::mem::transmute(self.calls.handle_slot_selection) };
        // SAFETY: the HUD controller of a tick less than HUD_FRESH_SECONDS ago that no destructor has run for
        unsafe { call(hud as *mut c_void, i32::from(slot), Self::INVENTORY_CONTAINER) };
        let selected = inventory.selected().unwrap_or(-1);
        if selected == i32::from(slot) {
            return true;
        }
        curia::warn!("easy place select not taken", { slot: slot, selected: selected });
        false
    }

    // clicks the neighbour's face that points at the spot, at the point the core chose, the way the game's own use
    // does; true when the game built
    #[allow(unsafe_code)]
    unsafe fn place(&self, signals: &UseSignals, player: *mut c_void, place: &Placement) -> bool {
        // SAFETY: the caller guarantees a live player
        let Some(game_mode) = (unsafe { GameMode::of_player(player, &self.players, &self.calls) }) else {
            return false;
        };
        // SAFETY: as above
        let Some(view) = (unsafe { PlayerView::of_player(player, &self.calls, &self.players) }) else { return false };
        let Some(hit) = view.hit_result() else { return false };
        let neighbour = BlockPos { x: place.neighbour[0], y: place.neighbour[1], z: place.neighbour[2] };
        let face = place.face.index() as u8;
        // SAFETY: the level's live HitResult, which the mirror lays out up to mPos; only those members change, and
        // they are put back after the build
        let saved = unsafe {
            let hit = &mut *hit;
            let saved = (hit.kind, hit.facing, hit.block, hit.pos);
            hit.kind = Self::TILE;
            hit.block = neighbour;
            hit.facing = face;
            hit.pos = [
                neighbour.x as f32 + place.click[0],
                neighbour.y as f32 + place.click[1],
                neighbour.z as f32 + place.click[2],
            ];
            saved
        };
        signals.set_own_build(true);
        let built = game_mode.build_block(&neighbour, face, GameMode::MAIN_HAND, signals.sim_tick());
        signals.set_own_build(false);
        // SAFETY: as above
        unsafe {
            let hit = &mut *hit;
            (hit.kind, hit.facing, hit.block, hit.pos) = saved;
        }
        built
    }
}
