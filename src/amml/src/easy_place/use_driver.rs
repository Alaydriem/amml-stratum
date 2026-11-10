use std::ffi::c_void;

use stratum_core::structs::easy_place::{GameMode as UseGameMode, InventorySlot, SwapAnswer, UseCommand, UseProbe};

use crate::easy_place::{InventoryReader, ProbeReader, SpotReader, StackRequest, UseExecutor, UseSignals};
use crate::game::{GameMode, PlayerInventory, PlayerView, WorldBlocks};
use crate::traits::UseCore;
use crate::types::{PacketCalls, PlayerCalls, UseCalls, UseFrame};

/// Runs the use calls of a hold. While the use key is down and no menu shows, it makes one use call per
/// `GameMode::buildingTickDelay` from the client tick (held repeats do not reach the game's build action), and one on
/// the press when the tick has not made it yet. Each call reads the probe, asks the core for a spot, reads the spot's
/// neighbours and the inventory, asks the core what to do and has the executor carry it out. On key up it tells the
/// core once. It keeps the time of the last call, the swap answer that call left and the status of the last call; the
/// hold flag, the hold's last decision and the local player of the hold are in the signals the build hook reads.
pub struct UseDriver {
    calls: UseCalls,
    players: PlayerCalls,
    blocks: WorldBlocks,
    executor: UseExecutor,
    request: StackRequest,
    last_call: f64,
    // the swap answer as the last call left it
    last_answer: SwapAnswer,
    // whether the last use call reached the core, so a failure and a recovery are each logged once
    last_ok: bool,
}

// SAFETY: the stack request's raw pointers are game objects used on the game thread only, under the runtime's lock
#[allow(unsafe_code)]
unsafe impl Send for UseDriver {}

impl UseDriver {
    const MS_PER_SECOND: f64 = 1000.0;

    pub fn new(packets: &PacketCalls, players: &PlayerCalls, calls: &UseCalls) -> Self {
        Self {
            calls: *calls,
            players: *players,
            blocks: WorldBlocks::new(calls),
            executor: UseExecutor::new(calls, players),
            request: StackRequest::new(packets, players, calls),
            last_call: 0.0,
            last_answer: SwapAnswer::None,
            last_ok: true,
        }
    }

    /// The item stack request of the hold.
    pub fn request(&mut self) -> &mut StackRequest {
        &mut self.request
    }

    /// After the game's client update. `core` is None while no core runs.
    ///
    /// # Safety
    /// The frame's client, player and region must be live, or null.
    #[allow(unsafe_code)]
    pub unsafe fn tick(
        &mut self,
        signals: &UseSignals,
        core: Option<&mut dyn UseCore>,
        frame: &UseFrame,
        palette: &mut dyn FnMut(u16) -> *const c_void,
    ) {
        self.request.expire(frame.now);
        if frame.player.is_null() {
            signals.set_player(0);
        }
        // SAFETY: the caller guarantees a null or live player
        let game_mode = unsafe { GameMode::of_player(frame.player, &self.players, &self.calls) };
        let Some(core) = core else {
            Self::release(signals, None);
            return;
        };
        let (Some(game_mode), false, true) = (game_mode, frame.region.is_null(), frame.key_held) else {
            Self::release(signals, Some(core));
            return;
        };
        if signals.holding() && !self.due(&game_mode, frame.now) {
            return;
        }
        // SAFETY: forwarded to the caller
        unsafe { self.call(signals, core, frame, !signals.holding(), palette) };
    }

    /// `ClientInputCallbacks::handleBuildAction`, before the game: true cancels the game's handling of the press.
    ///
    /// # Safety
    /// As [`UseDriver::tick`].
    #[allow(unsafe_code)]
    pub unsafe fn press(
        &mut self,
        signals: &UseSignals,
        core: Option<&mut dyn UseCore>,
        frame: &UseFrame,
        palette: &mut dyn FnMut(u16) -> *const c_void,
    ) -> bool {
        let Some(core) = core.filter(|_| frame.key_held) else { return false };
        // SAFETY: the caller guarantees a null or live player
        let game_mode = unsafe { GameMode::of_player(frame.player, &self.players, &self.calls) };
        if signals.holding() && !game_mode.is_some_and(|game_mode| self.due(&game_mode, frame.now)) {
            return signals.blocking();
        }
        // SAFETY: forwarded to the caller
        unsafe { self.call(signals, core, frame, !signals.holding(), palette) != UseCommand::Vanilla }
    }

    /// `GameMode::buildBlock`, before the game: true skips the game's own build for the local player, because the
    /// hold's last decision was not to leave the use to the game. The executor's own build always goes through, and so
    /// does every build of another player's game mode, such as the local world's server side. Any thread.
    pub fn blocks_game_build(signals: &UseSignals, player: usize, is_sim_tick: bool) -> bool {
        if player == 0 || player != signals.player() || signals.own_build() {
            return false;
        }
        signals.set_sim_tick(is_sim_tick);
        signals.holding() && signals.blocking()
    }

    /// Ends the hold without telling the core and clears the executor and the request in flight; after the hooks are
    /// removed on mod disable.
    pub fn reset(&mut self, signals: &UseSignals) {
        signals.set_holding(false);
        signals.set_blocking(false);
        self.last_call = 0.0;
        self.last_answer = SwapAnswer::None;
        self.last_ok = true;
        signals.set_player(0);
        self.request.reset();
        UseExecutor::reset(signals);
    }

    /// The key went up or the hold cannot go on: forgets the player and ends the hold, telling `core` once.
    pub fn release(signals: &UseSignals, core: Option<&mut dyn UseCore>) {
        signals.set_player(0);
        if !signals.holding() {
            return;
        }
        signals.set_holding(false);
        signals.set_blocking(false);
        if let Some(core) = core {
            core.released();
        }
    }

    // a call is due once the build delay has passed since the last call, or as soon as the answer to a swap or pick
    // has come, so the block is placed on the server's answer
    fn due(&self, game_mode: &GameMode, now: f64) -> bool {
        let delay = game_mode.building_tick_delay_ms() as f64 / Self::MS_PER_SECOND;
        if now - self.last_call >= delay {
            return true;
        }
        self.last_answer == SwapAnswer::None && self.request.answer() != SwapAnswer::None
    }

    // one use call; returns the command
    #[allow(unsafe_code)]
    unsafe fn call(
        &mut self,
        signals: &UseSignals,
        core: &mut dyn UseCore,
        frame: &UseFrame,
        pressed: bool,
        palette: &mut dyn FnMut(u16) -> *const c_void,
    ) -> UseCommand {
        // SAFETY: the caller guarantees a null or live player
        let game_mode = unsafe { GameMode::of_player(frame.player, &self.players, &self.calls) };
        // SAFETY: as above
        let view = unsafe { PlayerView::of_player(frame.player, &self.calls, &self.players) };
        let (Some(game_mode), Some(view), false) = (game_mode, view, frame.region.is_null()) else {
            return UseCommand::Vanilla;
        };
        let mut probe = UseProbe {
            time_seconds: frame.now,
            pressed,
            held: true,
            eye: [0.0; 3],
            dir: [0.0; 3],
            reach: 0.0,
            game_mode: UseGameMode::Other,
            hit: None,
            body: None,
            swap_answer: SwapAnswer::None,
        };
        // SAFETY: the caller guarantees a live region of the player's world
        unsafe { ProbeReader::read(&view, &self.blocks, frame.region, &game_mode, &mut probe) };
        probe.swap_answer = self.request.answer();
        let aim = core.aim(&probe);
        // SAFETY: as above
        let world = aim.flatten().map(|spot| unsafe { SpotReader::read(&self.blocks, frame.region, spot) });
        let command = match aim {
            Some(_) => {
                // SAFETY: the caller guarantees a live player
                let inventory = unsafe { PlayerInventory::of_player(frame.player, &self.players) };
                let (slots, count, selected) = match inventory.as_ref() {
                    Some(inventory) => InventoryReader::read(inventory),
                    None => ([InventorySlot { item: &[], count: 0 }; InventoryReader::CAPACITY], 0, 0),
                };
                core.decide(&probe, world.as_ref(), &slots[..count], selected)
            }
            None => None,
        };
        let ok = command.is_some();
        let command = command.unwrap_or(UseCommand::Vanilla);
        if ok != self.last_ok {
            if ok {
                curia::info!("easy place use call recovered", {});
            } else {
                curia::warn!("easy place use call failed", {});
            }
            self.last_ok = ok;
        }
        signals.set_player(frame.player as usize);
        signals.set_holding(true);
        self.last_call = frame.now;
        signals.set_blocking(command != UseCommand::Vanilla);
        // SAFETY: forwarded to the caller
        unsafe {
            self.executor.run(
                signals,
                &mut self.request,
                core,
                frame.now,
                frame.client,
                frame.player,
                &command,
                palette,
            )
        };
        self.last_answer = self.request.answer();
        command
    }
}
