use std::sync::Arc;

use alaydriem_stratum::services::{FrameRouter, ManagerSession, SharedUi, StratumHandle};
use amm_client::test_support::{FakeConnection, FakeTable};

use super::LoggedLine;
use amm_core::structs::event::{KeybindPhase, ModEvent};
use amml_api::testing::FakeGame;
use stratum_core::structs::frame::FrameStatus;
use tempfile::TempDir;

// A live core registered on this thread's fake manager, with its own data folder, and the fake loader its link runs
// on. Shut down on drop, before the loader.
pub struct CoreSession {
    pub handle: Option<StratumHandle>,
    pub data: TempDir,
    pub game: FakeGame,
}

impl CoreSession {
    pub const WORLD: &'static str = "Survival One";
    // a key no descriptor passes through: A
    pub const PROBE_KEY: u16 = 0x41;
    pub const OPEN_KEY: ModEvent = ModEvent::Keybind { action: 0, phase: KeybindPhase::Pressed };

    pub fn new() -> Self {
        FakeTable::set_config("");
        let data = TempDir::new().unwrap();
        let shared = StratumHandle::shared_for(data.path());
        let FakeConnection { link, game } =
            FakeTable::link_with(ManagerSession::DESCRIPTOR, FrameRouter::new(Arc::clone(&shared)));
        let input = FakeTable::ctx_of(&game, ManagerSession::DESCRIPTOR).input();
        let handle = StratumHandle::new(link, shared, data.path(), input);
        Self { handle: Some(handle), data, game }
    }

    // In the overworld of `WORLD`, no menu, feet at (10, 64, -20), render distance 12 chunks.
    pub fn in_world() -> FrameStatus {
        FrameStatus {
            in_world: true,
            has_player: true,
            menu_open: false,
            world: Self::WORLD.to_string(),
            dimension: 0,
            feet: [10, 64, -20],
            time_seconds: 1.0,
            render_distance: 12,
        }
    }

    // In the world at game time `time_seconds`.
    pub fn in_world_at(time_seconds: f64) -> FrameStatus {
        FrameStatus { time_seconds, ..Self::in_world() }
    }

    // In the world with the pause screen or another game menu shown.
    pub fn in_menu() -> FrameStatus {
        FrameStatus { menu_open: true, ..Self::in_world() }
    }

    // In the world at game time `time_seconds` with a game menu shown.
    pub fn in_menu_at(time_seconds: f64) -> FrameStatus {
        FrameStatus { menu_open: true, ..Self::in_world_at(time_seconds) }
    }

    pub fn out_of_world() -> FrameStatus {
        FrameStatus { in_world: false, has_player: false, world: String::new(), ..Self::in_world() }
    }

    // None after a panic poisoned the core's lock.
    pub fn tick(&self, status: &FrameStatus) -> Option<()> {
        self.handle().with_state(|state| state.tick(status))
    }

    pub fn handle(&self) -> &StratumHandle {
        self.handle.as_ref().expect("the session is shut down")
    }

    pub fn shared(&self) -> Arc<SharedUi> {
        Arc::clone(self.handle().shared())
    }

    // The open key delivered on an in-world tick.
    pub fn open(&self) {
        FakeTable::push_event(Self::OPEN_KEY);
        assert_eq!(self.tick(&Self::in_world()), Some(()));
    }

    // Whether a lease of this mod blocks the player's keys: a real key that is not passed through does not reach the
    // game. The key goes up again at once.
    pub fn input_held(&self) -> bool {
        let reached = self.game.real_key(Self::PROBE_KEY, true);
        self.game.real_key(Self::PROBE_KEY, false);
        !reached
    }

    // The mod's log lines whose text holds `fragment`, as `<level> <message> key=value ...`.
    pub fn logged(&self, fragment: &str) -> Vec<LoggedLine> {
        self.game.logs().into_iter().filter(|line| line.contains(fragment)).map(LoggedLine::new).collect()
    }

    // Shuts the core down now; the data folder stays until the session drops.
    pub fn destroy(&mut self) {
        if let Some(handle) = self.handle.take() {
            handle.shutdown();
        }
    }
}

impl Drop for CoreSession {
    fn drop(&mut self) {
        self.destroy();
    }
}
