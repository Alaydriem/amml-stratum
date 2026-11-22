use amml_api::{ItemSnapshot, LocalPlayer, RequestId, RequestOutcome};
use stratum_core::structs::easy_place::{GameMode as UseGameMode, SwapAnswer, UseCommand, UseProbe};

use crate::easy_place::{InventoryReader, ProbeReader, RequestTracker, SpotReader, UseFlags};
use crate::traits::UseCore;
use crate::types::UseOutcome;

/// Decides the use calls of a hold. While the use key is down and no menu shows, it makes one use call per build delay
/// from the client tick (held repeats do not reach the game's use press), and one on the press when the tick has not
/// made it yet. Each call reads the probe, asks the core for a spot, reads the spot's neighbours and the inventory and
/// asks the core what to do. The caller carries the command out with `UseExecutor` after it has released its locks,
/// then hands the outcome back with `apply`. On key up it tells the core once. The hold flags are in `UseFlags`, which
/// the `BuildBlock` listener reads.
///
/// Bound: `slots` is the one inventory buffer, 36 entries, reused by every call.
pub struct UseDriver {
    tracker: RequestTracker,
    last_call: f64,
    // the swap answer as the last call left it
    last_answer: SwapAnswer,
    // whether the last use call reached the core, so a failure and a recovery are each logged once
    last_ok: bool,
    slots: [Option<ItemSnapshot>; InventoryReader::CAPACITY],
}

impl UseDriver {
    const MS_PER_SECOND: f64 = 1000.0;
    // the game's build repeat delay, for a game mode whose delay the loader cannot read
    const DEFAULT_DELAY_MS: u32 = 200;

    pub fn new() -> Self {
        Self {
            tracker: RequestTracker::new(),
            last_call: 0.0,
            last_answer: SwapAnswer::None,
            last_ok: true,
            slots: std::array::from_fn(|_| None),
        }
    }

    /// The stack counts of the inventory the last call decided on, for the executor.
    pub fn counts(&self) -> [u8; InventoryReader::CAPACITY] {
        std::array::from_fn(|slot| self.slots[slot].as_ref().map_or(0, |item| item.count))
    }

    /// The request the hold waits for.
    pub fn awaited(&self) -> Option<RequestId> {
        self.tracker.awaited()
    }

    /// The client tick: a use call when one is due; None when the hold makes no call this tick. `player` is None
    /// with no local player; `key_held` is the real use key with no menu shown.
    pub fn tick(
        &mut self,
        flags: &UseFlags,
        core: Option<&mut dyn UseCore>,
        player: Option<&LocalPlayer<'_>>,
        key_held: bool,
        now: f64,
    ) -> Option<UseCommand> {
        let Some(core) = core else {
            Self::release(flags, None);
            return None;
        };
        let (Some(player), true) = (player, key_held) else {
            Self::release(flags, Some(core));
            return None;
        };
        if flags.holding() && !self.due(player, now) {
            return None;
        }
        self.call(flags, core, player, !flags.holding(), now)
    }

    /// The use press, before the game: the command of a call made now, or, while a hold waits for its next call,
    /// `Block` when the hold blocks the game's use and None when it does not.
    pub fn press(
        &mut self,
        flags: &UseFlags,
        core: Option<&mut dyn UseCore>,
        player: Option<&LocalPlayer<'_>>,
        key_held: bool,
        now: f64,
    ) -> Option<UseCommand> {
        let (Some(core), Some(player), true) = (core, player, key_held) else { return None };
        if flags.holding() && !self.due(player, now) {
            return flags.blocking().then_some(UseCommand::Block);
        }
        self.call(flags, core, player, !flags.holding(), now)
    }

    /// What carrying out the last command gave: a placement the game did not make goes to the core, and the loader's
    /// answer to a swap or pick to the tracker.
    pub fn apply(&mut self, outcome: &UseOutcome, core: Option<&mut dyn UseCore>) {
        if let Some(request) = outcome.request {
            self.tracker.sent(request);
        }
        if let (Some(spot), Some(core)) = (outcome.not_sent, core) {
            core.not_sent(spot);
        }
        self.last_answer = self.tracker.answer();
    }

    /// An `InventoryRequestDone`.
    pub fn request_done(&mut self, id: RequestId, outcome: RequestOutcome) {
        self.tracker.done(id, outcome);
    }

    /// Ends the hold without telling the core and forgets the request in flight: a world leave or a disable.
    pub fn reset(&mut self, flags: &UseFlags) {
        flags.set_holding(false);
        flags.set_blocking(false);
        self.last_call = 0.0;
        self.last_answer = SwapAnswer::None;
        self.last_ok = true;
        self.tracker.reset();
    }

    /// The key went up or the hold cannot go on: ends the hold, telling `core` once.
    pub fn release(flags: &UseFlags, core: Option<&mut dyn UseCore>) {
        if !flags.holding() {
            return;
        }
        flags.set_holding(false);
        flags.set_blocking(false);
        if let Some(core) = core {
            core.released();
        }
    }

    // a call is due once the build delay has passed since the last call, or as soon as the answer to a swap or pick
    // has come, so the block is placed on the server's answer
    fn due(&self, player: &LocalPlayer<'_>, now: f64) -> bool {
        let delay_ms =
            player.game_mode().and_then(|game_mode| game_mode.build_delay()).unwrap_or(Self::DEFAULT_DELAY_MS);
        if now - self.last_call >= f64::from(delay_ms) / Self::MS_PER_SECOND {
            return true;
        }
        self.last_answer == SwapAnswer::None && self.tracker.answer() != SwapAnswer::None
    }

    // one use call; the command to carry out
    fn call(
        &mut self,
        flags: &UseFlags,
        core: &mut dyn UseCore,
        player: &LocalPlayer<'_>,
        pressed: bool,
        now: f64,
    ) -> Option<UseCommand> {
        let (Some(game_mode), Some(source)) = (player.game_mode(), player.block_source()) else {
            return Some(UseCommand::Vanilla);
        };
        let mut probe = UseProbe {
            time_seconds: now,
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
        ProbeReader::read(player, &source, &game_mode, &mut probe);
        probe.swap_answer = self.tracker.answer();
        let aim = core.aim(&probe);
        let world = aim.flatten().map(|spot| SpotReader::read(&source, spot));
        let command = match aim {
            Some(_) => {
                let inventory = player.inventory();
                let selected = InventoryReader::read(inventory.as_ref(), &mut self.slots);
                let slots = InventoryReader::slots(&self.slots);
                core.decide(&probe, world.as_ref(), &slots, selected)
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
        flags.set_holding(true);
        flags.set_blocking(command != UseCommand::Vanilla);
        self.last_call = now;
        self.last_answer = self.tracker.answer();
        Some(command)
    }
}

impl Default for UseDriver {
    fn default() -> Self {
        Self::new()
    }
}
