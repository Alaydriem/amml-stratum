use std::path::Path;
use std::sync::Arc;

use alaydriem_stratum::services::{SharedUi, StratumHandle};
use amm_client::test_support::{FakeTable, LoggedCall};
use amm_core::structs::event::{KeybindPhase, ModEvent};
use stratum_core::structs::frame::FrameStatus;
use tempfile::TempDir;

// A live core registered on this thread's fake manager, with its own data folder. Shut down on drop.
pub struct CoreSession {
    pub handle: Option<StratumHandle>,
    pub data: TempDir,
}

impl CoreSession {
    pub const WORLD: &'static str = "Survival One";
    pub const OPEN_KEY: ModEvent = ModEvent::Keybind { action: 0, phase: KeybindPhase::Pressed };

    pub fn new() -> Self {
        FakeTable::set_config("");
        let data = TempDir::new().unwrap();
        let handle = StratumHandle::connect(FakeTable::table(), Path::new(FakeTable::CONFIG_DIR), data.path()).unwrap();
        Self { handle: Some(handle), data }
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

    pub fn logged(fragment: &str) -> Vec<LoggedCall> {
        FakeTable::with(|state| state.logs.iter().filter(|call| call.message.contains(fragment)).cloned().collect())
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
