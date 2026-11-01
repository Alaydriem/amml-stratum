use std::ffi::c_void;
use std::sync::{Mutex, OnceLock, PoisonError};

use amml_game::{ClientInstance, ContainerClosePacketLayout, ContainerOpenPacketLayout, ItemStackResponsePacketLayout};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_RBUTTON};

use crate::easy_place::{UseDriver, UseExecutor, UseSignals};
use crate::game::GameMode;
use crate::traits::UseCore;
use crate::types::{PacketCalls, PlayerCalls, UseCalls, UseFrame};

/// What the easy place detours reach, with no logic of its own: each hook body takes the use lock or reads the
/// signals and calls the service the C++ hook called. The tick and the press take the lock; the packet hooks only try
/// it, so a packet that comes while a use call runs goes to the game; the build and HUD hooks read the signals only.
pub struct UseHooks {
    signals: UseSignals,
    calls: OnceLock<UseCalls>,
    driver: Mutex<Option<UseDriver>>,
}

impl UseHooks {
    // the high bit of GetAsyncKeyState: the key is down now
    const KEY_DOWN: u16 = 0x8000;

    pub const fn new() -> Self {
        Self { signals: UseSignals::new(), calls: OnceLock::new(), driver: Mutex::new(None) }
    }

    pub fn signals(&self) -> &UseSignals {
        &self.signals
    }

    /// Enable, with every easy place entry: a driver for the calls, or the one of the last enable.
    pub fn start(&self, packets: &PacketCalls, players: &PlayerCalls, calls: &UseCalls) {
        let _ = self.calls.set(*calls);
        let mut driver = self.driver.lock().unwrap_or_else(PoisonError::into_inner);
        if driver.is_none() {
            *driver = Some(UseDriver::new(packets, players, calls));
        }
    }

    /// Disable, after the hooks are removed: the hold ends and the request in flight is undone.
    pub fn reset(&self) {
        if let Some(driver) = self.driver.lock().unwrap_or_else(PoisonError::into_inner).as_mut() {
            driver.reset(&self.signals);
        }
    }

    /// The frame of `client`: its local player and region, and whether the right mouse button is down with no menu
    /// showing.
    pub fn frame(client: &ClientInstance<'_>, now: f64) -> UseFrame {
        let player = client.local_player().map_or(std::ptr::null_mut(), |player| player.as_ptr());
        let region = client.region().map_or(std::ptr::null_mut(), |region| region.as_ptr());
        // SAFETY: GetAsyncKeyState reads the key state and has no other effect
        #[allow(unsafe_code)]
        let down = (unsafe { GetAsyncKeyState(i32::from(VK_RBUTTON.0)) } as u16 & Self::KEY_DOWN) != 0;
        UseFrame { client: client.as_ptr(), player, region, key_held: down && !client.is_showing_menu(), now }
    }

    /// The client tick, after the game's update.
    ///
    /// # Safety
    /// The frame's client, player and region must be live, or null.
    #[allow(unsafe_code)]
    pub unsafe fn tick(
        &self,
        core: Option<&mut dyn UseCore>,
        frame: &UseFrame,
        palette: &mut dyn FnMut(u16) -> *const c_void,
    ) {
        let Ok(mut driver) = self.driver.lock() else { return };
        let Some(driver) = driver.as_mut() else { return };
        // SAFETY: forwarded to the caller
        unsafe { driver.tick(&self.signals, core, frame, palette) };
    }

    /// The use press: true cancels the game's handling of it.
    ///
    /// # Safety
    /// As [`UseHooks::tick`].
    #[allow(unsafe_code)]
    pub unsafe fn press(
        &self,
        core: Option<&mut dyn UseCore>,
        frame: &UseFrame,
        palette: &mut dyn FnMut(u16) -> *const c_void,
    ) -> bool {
        let Ok(mut driver) = self.driver.try_lock() else { return false };
        let Some(driver) = driver.as_mut() else { return false };
        // SAFETY: forwarded to the caller
        unsafe { driver.press(&self.signals, core, frame, palette) }
    }

    /// A game mode's build: true skips the game's own build. Any thread.
    ///
    /// # Safety
    /// `game_mode` must be the live `GameMode` of the running build.
    #[allow(unsafe_code)]
    pub unsafe fn blocks_game_build(&self, game_mode: *mut c_void, is_sim_tick: bool) -> bool {
        let Some(calls) = self.calls.get() else { return false };
        // SAFETY: forwarded to the caller
        let Some(game_mode) = (unsafe { GameMode::from_raw(game_mode, calls) }) else { return false };
        UseDriver::blocks_game_build(&self.signals, game_mode.player(), is_sim_tick)
    }

    /// The HUD screen's tick.
    pub fn hud_tick(&self, hud: usize, now: f64) {
        UseExecutor::capture_hud(&self.signals, hud, now);
    }

    /// A screen controller's destructor, before the game's.
    pub fn screen_gone(&self, controller: usize) {
        UseExecutor::forget_hud(&self.signals, controller);
    }

    /// The server's item stack responses.
    pub fn on_response(&self, packet: &ItemStackResponsePacketLayout) {
        let Ok(mut driver) = self.driver.try_lock() else { return };
        if let Some(driver) = driver.as_mut() {
            driver.request().on_response(packet);
        }
    }

    /// The server's container open: true hides it from the game.
    pub fn on_container_open(&self, now: f64, packet: &ContainerOpenPacketLayout) -> bool {
        let Ok(mut driver) = self.driver.try_lock() else { return false };
        driver.as_mut().is_some_and(|driver| driver.request().on_container_open(now, packet))
    }

    /// The server's container close: true hides it from the game.
    pub fn on_container_close(&self, now: f64, packet: &ContainerClosePacketLayout) -> bool {
        let Ok(mut driver) = self.driver.try_lock() else { return false };
        driver.as_mut().is_some_and(|driver| driver.request().on_container_close(now, packet))
    }
}

impl Default for UseHooks {
    fn default() -> Self {
        Self::new()
    }
}
