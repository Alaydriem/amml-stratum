use alaydriem_stratum::StratumMod;
use amm_client::link::LinkUses;
use amml_api::amml_abi::{
    AmmlMouseAction, AmmlMouseButton, AmmlOrigin, AmmlThread, MouseInput, WindowFocus, WorldJoined, WorldLeft,
};
use amml_api::testing::FakeGame;
use amml_api::{AmmlMod, BlockPos, Face, Hand};

use crate::support::StratumRig;

#[test]
fn the_uses_block_has_every_row_the_manager_link_needs() {
    assert_eq!(LinkUses::check(&StratumMod::USES), Ok(()));
}

#[test]
fn the_checked_in_manifest_holds_the_uses_block_of_the_mod() {
    let manifest = include_str!("../amml.hcl");

    assert_eq!(StratumMod::USES.write_into(manifest), manifest);
}

#[test]
fn every_row_of_the_uses_block_is_required() {
    assert!(StratumMod::USES.rows().iter().all(|row| row.required != 0));
}

#[test]
fn enable_logs_one_feature_line_for_each_feature() {
    let rig = StratumRig::new();

    for feature in StratumMod::FEATURES {
        assert_eq!(rig.lines(&format!("info feature on name={feature}")).len(), 1, "{feature}");
    }
}

#[test]
fn the_core_starts_once_at_the_first_tick_after_the_link_connects() {
    let rig = StratumRig::new();
    rig.ready();
    assert!(rig.lines("stratum linked").is_empty(), "no core starts in enable");

    for _ in 0..3 {
        rig.tick();
    }

    assert_eq!(rig.lines("stratum linked").len(), 1);
}

#[test]
fn a_world_join_is_logged_with_its_epoch() {
    let rig = StratumRig::new();

    rig.game.raise(&mut WorldJoined { header: FakeGame::header::<WorldJoined>(), epoch: 4 });

    assert_eq!(rig.lines("info world joined epoch=4").len(), 1);
}

#[test]
fn a_local_world_is_keyed_by_its_storage_folder_and_the_build_conditions_are_logged_in_order() {
    let rig = StratumRig::new();
    rig.ready();

    for _ in 0..3 {
        rig.tick();
    }

    let key = rig.lines("world key kind=local");
    assert_eq!(key.len(), 1, "{:?}", rig.game.logs());
    assert!(key[0].contains(&format!("key=local:{}", StratumRig::FOLDER)), "{key:?}");
    assert_eq!(rig.lines("info build conditions local=true cheats=true operator=true creative=true").len(), 1);
    let conditions = rig.lines("info build conditions local=true");
    assert!(conditions[0].contains("connection=Some(Local)"), "{conditions:?}");
    assert!(conditions[0].contains("game_type=Some(Creative)"), "{conditions:?}");
    assert!(conditions[0].contains("permission_level=Some(Operator)"), "{conditions:?}");
}

#[test]
fn the_focus_loss_reaches_the_game_while_no_picker_is_open() {
    let rig = StratumRig::new();
    rig.ready();
    rig.tick();

    assert!(!rig.game.raise_app_focus_lost());
}

#[test]
fn a_build_on_the_server_thread_or_of_the_mod_itself_is_never_cancelled() {
    let rig = StratumRig::new();
    rig.ready();
    rig.tick();
    let pos = BlockPos { x: 1, y: 64, z: 1 };

    let server =
        rig.game.raise_build_block(AmmlThread::SERVER, AmmlOrigin::GAME, pos, Face::Up, Hand::Main, true, false);
    let own = rig.game.raise_build_block(AmmlThread::GAME, AmmlOrigin::THIS_MOD, pos, Face::Up, Hand::Main, true, true);
    let game = rig.game.raise_build_block(AmmlThread::GAME, AmmlOrigin::GAME, pos, Face::Up, Hand::Main, true, true);

    assert!(!server);
    assert!(!own);
    assert!(!game, "no hold blocks the game's build");
}

#[test]
fn a_press_with_no_ghost_goes_to_the_game() {
    let rig = StratumRig::new();
    rig.ready();
    rig.tick();
    let mut right = MouseInput {
        header: FakeGame::header::<MouseInput>(),
        action: AmmlMouseAction::BUTTON,
        button: AmmlMouseButton::RIGHT,
        down: 1,
        _reserved: 0,
        x: 0,
        y: 0,
        dx: 0,
        dy: 0,
        notches: 0.0,
    };
    rig.game.raise(&mut right);
    rig.tick();

    assert!(!rig.game.raise_use_pressed());
    assert!(rig.game.builds().is_empty());
}

#[test]
fn a_world_leave_and_a_focus_loss_are_taken_without_a_core() {
    let rig = StratumRig::new();

    rig.game.raise(&mut WindowFocus {
        header: FakeGame::header::<WindowFocus>(),
        gained: 0,
        replayed: 0,
        _reserved: [0; 2],
    });
    rig.game.raise(&mut WorldLeft { header: FakeGame::header::<WorldLeft>(), epoch: 1 });

    assert!(rig.lines("panic").is_empty());
}

#[test]
fn a_server_world_is_keyed_by_the_address_the_player_typed_not_the_resolved_host() {
    use amml_api::testing::FakeClient;
    use amml_api::{Connection, ConnectionKind, WorldInfo};
    let rig = StratumRig::new();
    rig.game.offer::<amml_api::amml_abi::ServerJoinStarting>();
    rig.game.raise_server_join_starting("203.0.113.7", "play.example.net", "Example", 19132, false);
    let info = WorldInfo {
        storage_folder: None,
        connection: Some(Connection { kind: ConnectionKind::Remote, host: "203.0.113.7".to_string(), port: 19132 }),
        ..StratumRig::client().world_info.unwrap()
    };
    rig.game.set_client(FakeClient { world_info: Some(info), ..StratumRig::client() });
    rig.ready();

    for _ in 0..=alaydriem_stratum::services::WorldKeyReader::SERVER_STABLE_READS {
        rig.tick();
    }

    let key = rig.lines("world key kind=server");
    assert_eq!(key.len(), 1, "{:?}", rig.game.logs());
    assert!(key[0].contains("key=server:play.example.net:19132"), "{key:?}");
}
