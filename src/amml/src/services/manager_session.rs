use std::collections::VecDeque;
use std::path::Path;
use std::sync::Arc;

use amm_client::link::{ApiTable, EventDrain, ManagerLink};
use amm_core::errors::LinkError;
use amm_core::structs::descriptor::Capability;
use amm_core::structs::event::{KeybindPhase, ModEvent};
use amm_core::structs::lease::AcquireOutcome;
use stratum_core::config::StratumConfig;
use stratum_core::structs::frame::FrameStatus;
use stratum_core::structs::hud::HudKey;

use super::{CloseReason, FrameRouter, SharedUi};

// The registration with the mod manager, on the game thread. It turns the manager's events into opening and
// closing the library, and holds the Overlay lease, the Input lease and the free cursor exactly while it is open.
// A denied Input lease leaves the library open, and the keys of the other mod stay live. It holds the WorldDraw
// lease while the ghost has a marker to draw, and the Hud lease while a marker is active with no game screen, no
// library and no Input held by another mod. A lease that is suspended, revoked or denied is asked for again
// `WORLD_DRAW_RETRY_SECONDS` or `HUD_RETRY_SECONDS` later, or at once on a resume. The HUD keybinds are live
// only while the Hud lease is held; their presses wait in a bounded queue for the tick. It reads the config
// again on each SettingsChanged and hands it to the tick.
pub struct ManagerSession {
    link: ManagerLink,
    shared: Arc<SharedUi>,
    // Events of one poll, at most one drain buffer, sized once in connect.
    pending: Vec<ModEvent>,
    is_open: bool,
    // False between a Disabled and an Enabled event.
    is_enabled: bool,
    holds_world_draw: bool,
    holds_input: bool,
    holds_hud: bool,
    // The earliest game time the Hud lease is asked for again.
    hud_retry_at: f64,
    // A Hud denial was logged and no grant came since.
    is_hud_denial_logged: bool,
    // Presses and repeats of the HUD keys since the tick last took them, at most `EventDrain::CAPACITY`.
    hud_keys: VecDeque<HudKey>,
    // One bit per HUD keybind action whose press came while the Hud lease was held and whose release has not.
    // Emptied when the lease goes.
    hud_held: u8,
    // The earliest game time the WorldDraw lease is asked for again.
    world_draw_retry_at: f64,
    // A denial was logged and no grant came since, so a repeated denial stays quiet.
    is_denial_logged: bool,
    // The lease was revoked, or the mod disabled, since the tick last asked: the ghost gives its meshes back.
    is_world_draw_revoked: bool,
    // The config read on the last SettingsChanged, until the tick takes it.
    changed_config: Option<StratumConfig>,
    // The file picker is open, or closed less than `PICKER_SETTLE_SECONDS` ago with a game menu still shown.
    is_picker_guarded: bool,
    // The picker was busy on the last tick, so the settle time starts on the first tick it is not.
    was_picker_busy: bool,
    // The game time the settle time ends.
    picker_settled_at: f64,
    // The guard ran out with a menu shown and closed the library; it opens again when the menu is gone.
    reopen_pending: bool,
}

impl ManagerSession {
    const DESCRIPTOR: &'static str = include_str!("../../descriptor.hcl");
    // The index of the `open_library` keybind in the descriptor.
    pub const OPEN_LIBRARY_ACTION: u16 = 0;
    // The indexes of the HUD keybinds in the descriptor.
    pub const HUD_UP_ACTION: u16 = 1;
    pub const HUD_DOWN_ACTION: u16 = 2;
    pub const HUD_LEFT_ACTION: u16 = 3;
    pub const HUD_RIGHT_ACTION: u16 = 4;
    pub const HUD_MODE_ACTION: u16 = 5;
    pub const WORLD_DRAW_RETRY_SECONDS: f64 = 2.0;
    pub const HUD_RETRY_SECONDS: f64 = 2.0;
    pub const PICKER_SETTLE_SECONDS: f64 = 2.0;

    // Registers the mod with `FrameRouter` as its egui pass: the library on the Overlay layer, the HUD card on the
    // Hud layer. `config_dir` is the mod's own config
    // folder, an absolute path. The library opens only with the `open_library` key.
    pub fn connect(table: ApiTable, config_dir: &Path, shared: Arc<SharedUi>) -> Result<Self, LinkError> {
        let link = ManagerLink::new(table, Self::DESCRIPTOR, config_dir, FrameRouter::new(Arc::clone(&shared)))?;
        Ok(Self {
            link,
            shared,
            pending: Vec::with_capacity(EventDrain::CAPACITY),
            is_open: false,
            is_enabled: true,
            holds_world_draw: false,
            holds_input: false,
            holds_hud: false,
            hud_retry_at: f64::NEG_INFINITY,
            is_hud_denial_logged: false,
            hud_keys: VecDeque::with_capacity(EventDrain::CAPACITY),
            hud_held: 0,
            world_draw_retry_at: f64::NEG_INFINITY,
            is_denial_logged: false,
            is_world_draw_revoked: false,
            changed_config: None,
            is_picker_guarded: false,
            was_picker_busy: false,
            picker_settled_at: f64::NEG_INFINITY,
            reopen_pending: false,
        })
    }

    // The manager's checked config.hcl; None, with a warning, when it cannot be read.
    pub fn config(&self) -> Option<StratumConfig> {
        match self.link.config::<StratumConfig>() {
            Ok(config) => Some(config),
            Err(error) => {
                curia::warn!("config not read, the defaults apply", { error: error.to_string() });
                None
            }
        }
    }

    // Handles this tick's events, then a close the frame asked for, a world exit or a game menu. The library
    // opens only inside a world with no game menu shown, and a menu or the pause screen closes it, so it never
    // holds the cursor under a game screen. While the file picker is open, and just after it, a menu does not
    // close it: the picker takes the window focus and the game answers with the pause screen.
    pub fn sync(&mut self, status: &FrameStatus, picker_busy: bool) {
        let expired = self.update_guard(status, picker_busy);
        self.pending.clear();
        self.pending.extend(self.link.events());
        for position in 0..self.pending.len() {
            let event = self.pending[position];
            self.handle(event, status);
        }
        if self.shared.take_close_request() {
            self.close(CloseReason::Frame);
        }
        if !status.in_world {
            self.reopen_pending = false;
            self.close(CloseReason::WorldExit);
        } else if status.menu_open {
            if !self.is_picker_guarded {
                if expired && self.is_open {
                    self.reopen_pending = true;
                }
                self.close(CloseReason::Menu);
            }
        } else if self.reopen_pending {
            self.reopen_pending = false;
            self.open(false);
        }
    }

    // True while the library stays open through the picker and the game's pause screen is not wanted: from the
    // picker's start until the first tick without a game menu, or `PICKER_SETTLE_SECONDS` after it ended.
    pub fn pause_screen_unwanted(&self) -> bool {
        self.is_picker_guarded
    }

    // Moves the picker guard for this tick. True when the guard ran out its settle time with a menu still shown.
    fn update_guard(&mut self, status: &FrameStatus, picker_busy: bool) -> bool {
        if picker_busy {
            self.is_picker_guarded = true;
            self.was_picker_busy = true;
            return false;
        }
        if std::mem::take(&mut self.was_picker_busy) {
            self.picker_settled_at = status.time_seconds + Self::PICKER_SETTLE_SECONDS;
        }
        if !self.is_picker_guarded {
            return false;
        }
        if !status.menu_open {
            self.is_picker_guarded = false;
            return false;
        }
        if status.time_seconds >= self.picker_settled_at {
            self.is_picker_guarded = false;
            return true;
        }
        false
    }

    fn handle(&mut self, event: ModEvent, status: &FrameStatus) {
        match event {
            ModEvent::Keybind { action: Self::OPEN_LIBRARY_ACTION, phase: KeybindPhase::Pressed } => {
                self.reopen_pending = false;
                if self.is_open {
                    self.close(CloseReason::Key);
                } else if status.in_world && !status.menu_open {
                    self.open(true);
                }
            }
            ModEvent::Keybind { action, phase: KeybindPhase::Pressed | KeybindPhase::Repeat } if self.holds_hud => {
                if let Some(key) = Self::hud_key(action) {
                    self.hud_held |= Self::hud_bit(action);
                    if self.hud_keys.len() < EventDrain::CAPACITY {
                        self.hud_keys.push_back(key);
                    }
                }
            }
            ModEvent::Keybind { action, phase: KeybindPhase::Released } => self.hud_held &= !Self::hud_bit(action),
            ModEvent::Suspended { capability: Capability::Hud, .. } => {
                self.lose_hud(status.time_seconds, CloseReason::Suspended);
            }
            ModEvent::Revoked { capability: Capability::Hud, .. } => {
                self.lose_hud(status.time_seconds, CloseReason::Revoked);
            }
            ModEvent::Resumed(Capability::Hud) => self.hud_retry_at = f64::NEG_INFINITY,
            ModEvent::Revoked { capability: Capability::Input, .. } => self.holds_input = false,
            ModEvent::Suspended { capability: Capability::Overlay, .. } if self.is_picker_guarded => {
                self.shared.close();
            }
            ModEvent::Resumed(Capability::Overlay) if self.is_open => self.shared.show(),
            ModEvent::Suspended { capability: Capability::Overlay, .. } => self.close(CloseReason::Suspended),
            ModEvent::Revoked { capability: Capability::Overlay, .. } => self.close(CloseReason::Revoked),
            ModEvent::Disabled => {
                self.reopen_pending = false;
                self.close(CloseReason::Disabled);
                self.is_enabled = false;
                self.lose_world_draw(status.time_seconds, CloseReason::Disabled);
            }
            ModEvent::Enabled => self.is_enabled = true,
            ModEvent::Suspended { capability: Capability::WorldDraw, .. } => {
                self.lose_world_draw(status.time_seconds, CloseReason::Suspended);
            }
            ModEvent::Revoked { capability: Capability::WorldDraw, .. } => {
                self.lose_world_draw(status.time_seconds, CloseReason::Revoked);
            }
            ModEvent::Resumed(Capability::WorldDraw) => self.world_draw_retry_at = f64::NEG_INFINITY,
            ModEvent::SettingsChanged { .. } => {
                if let Some(config) = self.config() {
                    self.shared.meter().set_enabled(config.debug.telemetry);
                    self.changed_config = Some(config);
                }
                curia::info!(
                    "settings changed; the open key, telemetry, ghost and preview settings apply now, the mesh budget after a restart",
                    {}
                );
            }
            _ => {}
        }
    }

    fn hud_key(action: u16) -> Option<HudKey> {
        match action {
            Self::HUD_UP_ACTION => Some(HudKey::Up),
            Self::HUD_DOWN_ACTION => Some(HudKey::Down),
            Self::HUD_LEFT_ACTION => Some(HudKey::Left),
            Self::HUD_RIGHT_ACTION => Some(HudKey::Right),
            Self::HUD_MODE_ACTION => Some(HudKey::Mode),
            _ => None,
        }
    }

    // The held bit of a HUD keybind action; zero for any other action.
    fn hud_bit(action: u16) -> u8 {
        match Self::hud_key(action) {
            Some(_) => 1 << (action - Self::HUD_UP_ACTION),
            None => 0,
        }
    }

    // True while a HUD key pressed under the Hud lease is still down. The manager releases every held key on a
    // focus loss, and losing the lease lets go of them all.
    pub fn hud_keys_held(&self) -> bool {
        self.hud_held != 0
    }

    // True while the Hud lease is held, so the HUD keys are live and the card shows.
    pub fn hud_shown(&self) -> bool {
        self.holds_hud
    }

    // The oldest HUD key press the tick has not handled.
    pub fn next_hud_key(&mut self) -> Option<HudKey> {
        self.hud_keys.pop_front()
    }

    // Holds the Hud lease exactly while a marker is active, the game is in a world with no game screen and no
    // library, no other mod holds Input and the mod is enabled. Anything else gives it back at once, which also
    // drops the queued keys. A shared Overlay of another mod does not matter. The manager has no query for an
    // exclusive Overlay held by another mod, so that case is not detected here.
    pub fn sync_hud(&mut self, has_marker: bool, status: &FrameStatus) {
        let live = has_marker
            && status.in_world
            && !status.menu_open
            && !self.is_open
            && self.is_enabled
            && !self.link.held_by_other(Capability::Input);
        if !live {
            self.hud_keys.clear();
            self.hud_held = 0;
            if std::mem::take(&mut self.holds_hud) {
                self.link.release(Capability::Hud);
                curia::info!("hud lease given back", {});
            }
            return;
        }
        if self.holds_hud || status.time_seconds < self.hud_retry_at {
            return;
        }
        match self.link.acquire(Capability::Hud) {
            Ok(AcquireOutcome::Granted | AcquireOutcome::Preempted) => {
                self.holds_hud = true;
                self.is_hud_denial_logged = false;
                curia::info!("hud lease held, the keys are live", {});
            }
            Ok(AcquireOutcome::Denied { .. }) => {
                if !std::mem::replace(&mut self.is_hud_denial_logged, true) {
                    let holder = self.link.holder(Capability::Hud).unwrap_or_default();
                    curia::info!("hud lease denied, the keys stay with the game", { holder: holder });
                }
                self.hud_retry_at = status.time_seconds + Self::HUD_RETRY_SECONDS;
            }
            Err(error) => {
                curia::warn!("hud lease request failed, the keys stay with the game", { error: error.to_string() });
                self.hud_retry_at = status.time_seconds + Self::HUD_RETRY_SECONDS;
            }
        }
    }

    // The keys stop now and the lease is asked for again later. A suspended lease is given back first; a revoked
    // one the manager already took back.
    fn lose_hud(&mut self, now: f64, reason: CloseReason) {
        self.hud_retry_at = now + Self::HUD_RETRY_SECONDS;
        self.hud_keys.clear();
        self.hud_held = 0;
        if !std::mem::take(&mut self.holds_hud) {
            return;
        }
        if reason != CloseReason::Revoked {
            self.link.release(Capability::Hud);
        }
        curia::info!("hud lease lost, the keys stop", { reason: reason.as_str() });
    }

    // Holds the WorldDraw lease exactly while `wanted` and the mod is enabled. True while the ghost may draw.
    pub fn sync_world_draw(&mut self, wanted: bool, now: f64) -> bool {
        if !wanted || !self.is_enabled {
            if std::mem::take(&mut self.holds_world_draw) {
                self.link.release(Capability::WorldDraw);
                curia::info!("world draw lease given back", {});
            }
            return false;
        }
        if self.holds_world_draw {
            return true;
        }
        if now < self.world_draw_retry_at {
            return false;
        }
        match self.link.acquire(Capability::WorldDraw) {
            Ok(AcquireOutcome::Granted | AcquireOutcome::Preempted) => {
                self.holds_world_draw = true;
                self.is_denial_logged = false;
                curia::info!("world draw lease held, the ghost draws", {});
            }
            Ok(AcquireOutcome::Denied { .. }) => {
                if !std::mem::replace(&mut self.is_denial_logged, true) {
                    let holder = self.link.holder(Capability::WorldDraw).unwrap_or_default();
                    curia::info!("world draw lease denied, the ghost waits", { holder: holder });
                }
                self.world_draw_retry_at = now + Self::WORLD_DRAW_RETRY_SECONDS;
            }
            Err(error) => {
                curia::warn!("world draw lease request failed, the ghost waits", { error: error.to_string() });
                self.world_draw_retry_at = now + Self::WORLD_DRAW_RETRY_SECONDS;
            }
        }
        self.holds_world_draw
    }

    // The config of the last SettingsChanged, once.
    pub fn take_changed_config(&mut self) -> Option<StratumConfig> {
        self.changed_config.take()
    }

    // True once after the WorldDraw lease was revoked or the mod disabled.
    pub fn take_world_draw_revoked(&mut self) -> bool {
        std::mem::take(&mut self.is_world_draw_revoked)
    }

    // The ghost stops now and the marker stays active; the lease is asked for again later. A suspended lease is
    // given back first; a revoked one the manager already took back. A revoke or a disable also asks the ghost to
    // give its meshes back; a suspension keeps them for the resume.
    fn lose_world_draw(&mut self, now: f64, reason: CloseReason) {
        self.world_draw_retry_at = now + Self::WORLD_DRAW_RETRY_SECONDS;
        if matches!(reason, CloseReason::Revoked | CloseReason::Disabled) {
            self.is_world_draw_revoked = true;
        }
        if !std::mem::take(&mut self.holds_world_draw) {
            return;
        }
        if reason != CloseReason::Revoked {
            self.link.release(Capability::WorldDraw);
        }
        curia::info!("world draw lease lost, the ghost stops", { reason: reason.as_str() });
    }

    // `fresh` starts the UI state anew; a reopen after the pause screen keeps the page the player was on.
    fn open(&mut self, fresh: bool) {
        if self.is_open || !self.take_overlay() {
            return;
        }
        if let Err(error) = self.link.set_cursor_mode(true) {
            curia::warn!("cursor not freed", { error: error.to_string() });
        }
        self.take_input();
        self.is_open = true;
        if fresh {
            self.shared.open();
        } else {
            self.shared.reopen();
        }
        curia::info!("library opened", {});
    }

    // True when the Overlay lease is held; a denial logs the holder.
    fn take_overlay(&self) -> bool {
        match self.link.acquire(Capability::Overlay) {
            Ok(AcquireOutcome::Granted | AcquireOutcome::Preempted) => true,
            Ok(AcquireOutcome::Denied { .. }) => {
                let holder = self.link.holder(Capability::Overlay).unwrap_or_default();
                curia::info!("overlay lease denied, the library stays closed", { holder: holder });
                false
            }
            Err(error) => {
                curia::warn!("lease request failed, the library stays closed", { error: error.to_string() });
                false
            }
        }
    }

    // Holds Input exclusively while the library is open. A denial leaves the library open and logs the holder.
    fn take_input(&mut self) {
        match self.link.acquire(Capability::Input) {
            Ok(AcquireOutcome::Granted | AcquireOutcome::Preempted) => self.holds_input = true,
            Ok(AcquireOutcome::Denied { .. }) => {
                let holder = self.link.holder(Capability::Input).unwrap_or_default();
                curia::info!("input lease denied, the library opens without it", { holder: holder });
            }
            Err(error) => {
                curia::warn!("input lease request failed, the library opens without it", { error: error.to_string() });
            }
        }
    }

    fn close(&mut self, reason: CloseReason) {
        self.shared.close();
        if std::mem::take(&mut self.is_open) {
            self.give_back();
            curia::info!("library closed", { reason: reason.as_str() });
        }
    }

    // After a panic: hides the library and gives back the lease and the cursor, whatever state the panic
    // left. A lease that is not held is ignored by the manager.
    pub fn release_ui(&mut self) {
        self.reopen_pending = false;
        self.shared.close();
        self.is_open = false;
        self.give_back();
        if std::mem::take(&mut self.holds_world_draw) {
            self.link.release(Capability::WorldDraw);
        }
        self.hud_keys.clear();
        self.hud_held = 0;
        if std::mem::take(&mut self.holds_hud) {
            self.link.release(Capability::Hud);
        }
    }

    fn give_back(&mut self) {
        if std::mem::take(&mut self.holds_input) {
            self.link.release(Capability::Input);
        }
        self.link.release(Capability::Overlay);
        if let Err(error) = self.link.set_cursor_mode(false) {
            curia::warn!("cursor not given back", { error: error.to_string() });
        }
    }
}
