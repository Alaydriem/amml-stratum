use std::sync::Arc;

use alaydriem_stratum::StratumMod;
use alaydriem_stratum::services::StratumRuntime;
use amm_client::test_support::FakeTable;
use amml_api::amml_abi::{AmmlDirKind, AmmlMouseAction, AmmlMouseButton, MouseInput};
use amml_api::testing::FakeGame;
use tempfile::TempDir;

fn right(down: bool, injected: bool) -> MouseInput {
    let mut header = FakeGame::header::<MouseInput>();
    header.injected = u8::from(injected);
    MouseInput {
        header,
        action: AmmlMouseAction::BUTTON,
        button: AmmlMouseButton::RIGHT,
        down: u8::from(down),
        _reserved: 0,
        x: 0,
        y: 0,
        dx: 0,
        dy: 0,
        notches: 0.0,
    }
}

// a runtime started on a fake loader with a data folder, outside the mod's lifecycle
struct RuntimeRig {
    game: FakeGame,
    _data: TempDir,
}

impl RuntimeRig {
    fn new() -> Self {
        let game = FakeTable::game();
        let data = TempDir::new().unwrap();
        game.set_dir(AmmlDirKind::DATA, data.path().to_str().unwrap());
        Self { game, _data: data }
    }

    fn start(&self) -> Arc<StratumRuntime> {
        StratumRuntime::start(&self.game.ctx_for::<StratumMod>()).unwrap()
    }
}

#[test]
fn the_use_key_is_the_real_right_button() {
    let rig = RuntimeRig::new();
    let runtime = rig.start();

    runtime.mouse(&right(true, true));
    assert!(!runtime.use_key(), "an injected press is not the player's");
    runtime.mouse(&right(true, false));
    assert!(runtime.use_key());
    runtime.mouse(&right(false, false));
    assert!(!runtime.use_key());
}

#[test]
fn a_focus_loss_and_a_world_leave_count_as_a_use_key_up() {
    let rig = RuntimeRig::new();
    let runtime = rig.start();

    runtime.mouse(&right(true, false));
    runtime.focus(false);
    assert!(!runtime.use_key());

    runtime.mouse(&right(true, false));
    runtime.focus(true);
    assert!(runtime.use_key(), "a gain changes nothing");
    runtime.world_left();
    assert!(!runtime.use_key());
}

#[test]
fn a_runtime_without_a_data_folder_refuses_the_enable() {
    let game = FakeTable::game();

    assert!(StratumRuntime::start(&game.ctx_for::<StratumMod>()).is_err());
    assert!(game.logs().iter().any(|line| line.contains("no data folder")));
}

#[test]
fn the_focus_loss_reaches_the_game_before_the_core_runs() {
    let rig = RuntimeRig::new();
    let runtime = rig.start();

    assert!(!runtime.pause_screen_unwanted());
}

#[test]
fn a_right_press_another_mod_cancelled_is_not_the_use_key() {
    let rig = RuntimeRig::new();
    let runtime = rig.start();
    let mut cancelled = right(true, false);
    cancelled.header.cancelled = 1;

    runtime.mouse(&cancelled);
    assert!(!runtime.use_key(), "the game got no press");
    runtime.mouse(&right(true, false));
    runtime.mouse(&{
        let mut up = right(false, false);
        up.header.cancelled = 1;
        up
    });
    assert!(!runtime.use_key(), "an up always ends the hold");
}
