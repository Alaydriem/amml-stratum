use amm_core::structs::descriptor::Capability;
use amm_core::structs::event::{KeybindPhase, ModEvent};
use stratum_core::structs::hud::HudState;

use super::CoreSession;

// Reads the HUD side of a session and sends it keys.
pub struct Hud;

impl Hud {
    pub fn count(calls: Vec<Capability>, capability: Capability) -> usize {
        calls.iter().filter(|call| **call == capability).count()
    }

    pub fn press(action: u16) -> ModEvent {
        ModEvent::Keybind { action, phase: KeybindPhase::Pressed }
    }

    pub fn shown(session: &CoreSession) -> bool {
        session.handle().with_state(|state| state.hud_shown()).unwrap()
    }

    pub fn release(action: u16) -> ModEvent {
        ModEvent::Keybind { action, phase: KeybindPhase::Released }
    }

    pub fn held(session: &CoreSession) -> bool {
        session.handle().with_state(|state| state.hud_keys_held()).unwrap()
    }

    pub fn state(session: &CoreSession) -> HudState {
        session.handle().with_state(|state| state.hud_state()).unwrap()
    }
}
