use std::time::{Duration, Instant};

use amm_client::test_support::FakeTable;
use amm_core::structs::event::{KeybindPhase, ModEvent};
use stratum_core::structs::easy_place::{GameMode, InventorySlot, SwapAnswer, UseProbe};

use super::{CoreSession, GhostScene};

// Steps for tests that place through the use path: the ghost scene with easy place switched on from the HUD keys.
pub struct EasyPlace;

impl EasyPlace {
    const DEADLINE: Duration = Duration::from_secs(60);
    pub const STONE: &'static str = "minecraft:stone";

    // The ghost scene, with the Mode, Left and Up keys pressed: Full mode, the Easy place control, toggle.
    pub fn start() -> CoreSession {
        let (session, _seen) = GhostScene::start();
        for action in [5, 3, 1] {
            FakeTable::push_event(ModEvent::Keybind { action, phase: KeybindPhase::Pressed });
            assert_eq!(session.tick(&CoreSession::in_world()), Some(()));
            FakeTable::push_event(ModEvent::Keybind { action, phase: KeybindPhase::Released });
        }
        assert_eq!(session.tick(&CoreSession::in_world()), Some(()));
        session
    }

    // Looks straight down at the column (x, z) from above the build.
    pub fn above(x: i32, z: i32) -> UseProbe {
        UseProbe {
            time_seconds: 1.0,
            pressed: false,
            held: true,
            eye: [f64::from(x) + 0.5, 70.5, f64::from(z) + 0.5],
            dir: [0.0, -1.0, 0.0],
            reach: 20.0,
            game_mode: GameMode::Survival,
            hit: None,
            body: None,
            swap_answer: SwapAnswer::None,
        }
    }

    pub fn stone_in_hand() -> [InventorySlot<'static>; 1] {
        [InventorySlot { item: Self::STONE.as_bytes(), count: 1 }]
    }

    // The ghost spot under the crosshair at `probe`.
    pub fn aim(session: &CoreSession, probe: &UseProbe) -> Option<[i32; 3]> {
        session.handle().with_state(|state| state.use_aim(probe)).unwrap()
    }

    // Ticks until the aim at `probe` names a spot, and returns it.
    pub fn aim_when_ready(session: &CoreSession, probe: &UseProbe) -> [i32; 3] {
        let start = Instant::now();
        loop {
            assert_eq!(session.tick(&CoreSession::in_world()), Some(()));
            if let Some(spot) = Self::aim(session, probe) {
                return spot;
            }
            assert!(start.elapsed() < Self::DEADLINE, "the ghost never gave a spot");
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}
