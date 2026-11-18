use std::sync::Arc;

use stratum_core::config::LibraryConfig;
use stratum_core::services::LibraryService;
use stratum_core::structs::action::{BuildAction, HudAction};
use stratum_core::structs::build::{BuildConditions, BuildPhase, BuildSettings, BuildStopReason};
use stratum_core::structs::ghost::GhostPiece;
use stratum_core::structs::hud::HudKey;
use stratum_core::structs::library::{Library, StructureId};
use stratum_core::structs::marker::{Axis, Rotation};
use stratum_core::structs::structure::StateValue;

use crate::support::{BlockFactory, BuildFixture, BuildScene, Frames, GhostFixture, Pieces};

const RESTORE: &str = "/gamerule sendcommandfeedback true";

#[test]
fn opening_a_structure_that_is_not_allowed_does_nothing() {
    let mut scene = BuildScene::new();
    scene.apply(BuildAction::Open(StructureId(2)));
    assert!(scene.service.row().is_none());

    scene.service.set_conditions(BuildConditions { creative: false, ..BuildScene::ALLOWED });
    scene.apply(BuildAction::Open(StructureId(1)));
    assert!(scene.service.row().is_none());

    scene.service.set_conditions(BuildScene::ALLOWED);
    scene.apply(BuildAction::Open(StructureId(1)));
    assert_eq!(scene.phase(), Some(BuildPhase::Probing));
}

#[test]
fn the_panel_waits_for_the_structure_then_asks_the_game_about_the_palette() {
    let mut scene = BuildScene::new();
    scene.apply(BuildAction::Open(StructureId(1)));

    // The structure is not loaded yet: nothing to ask. The service names the structure it waits for.
    let status = scene.status();
    scene.service.tick(&status, None, Some(&scene.markers), &scene.library);
    assert!(scene.service.take_probe().is_none());
    assert_eq!(scene.service.wants_structure(), Some(StructureId(1)));

    let request = scene.probe_request();
    assert!(!request.entries.is_empty());
    assert!(request.entries.iter().all(|(source, turned)| !source.is_empty() && !turned.is_empty()));
    assert!(scene.service.take_probe().is_none());
    assert_eq!(scene.phase(), Some(BuildPhase::Probing));

    scene.service.apply_probe(&BuildFixture::exact_entries(request.entries.len()));
    assert_eq!(scene.phase(), Some(BuildPhase::Probing));
    scene.settle();
    let row = scene.service.row().unwrap();
    assert_eq!(row.phase, BuildPhase::Ready);
    assert_eq!(row.structure_name, "piece 1");
    assert_eq!(row.total, 8);
}

#[test]
fn a_rotated_build_asks_for_and_places_the_turned_block() {
    let stairs = BlockFactory::block("oak_stairs", &[("weirdo_direction", StateValue::Int(0))]);
    let piece = GhostPiece { structure: GhostFixture::stateful(stairs), world_min: [0, 64, 0], remap: Vec::new() };
    let build = BuildFixture::build(piece, Rotation::R90);
    let mut scene = BuildScene::with_build(build, [1, 1, 1]);
    scene.markers.apply_hud(HudAction::Rotate(1), 1, &[]).unwrap();

    scene.apply(BuildAction::Open(StructureId(1)));
    let request = scene.probe_request();
    assert_ne!(request.entries[0].0, request.entries[0].1);
    scene.service.apply_probe(&BuildFixture::exact_entries(request.entries.len()));
    scene.settle();

    scene.apply(BuildAction::Start);
    let mut sent = scene.drain();
    sent.extend(scene.tick(0.25));
    sent.extend(scene.tick(1.0));
    assert!(sent.contains(&"/setblock 0 64 0 minecraft:oak_stairs [\"weirdo_direction\"=2]".to_string()));
}

#[test]
fn losing_a_condition_stops_and_restores_the_game_rule() {
    let mut scene = BuildScene::new();
    scene.running();
    scene.service.set_conditions(BuildConditions { operator: false, ..BuildScene::ALLOWED });

    let sent = scene.tick(1.0);

    assert_eq!(scene.phase(), Some(BuildPhase::Stopped(BuildStopReason::ConditionLost)));
    assert_eq!(sent.last().map(String::as_str), Some(RESTORE));
    assert_eq!(sent.len(), 1);
}

#[test]
fn leaving_the_world_stops_with_no_command() {
    let mut scene = BuildScene::new();
    scene.running();
    scene.tick(1.0);

    let sent = scene.tick_with(&Frames::main_menu(5.0));

    assert_eq!(scene.phase(), Some(BuildPhase::Stopped(BuildStopReason::LeftWorld)));
    assert!(sent.is_empty());
    // The finished build is forgotten once the world is gone.
    scene.tick_with(&Frames::main_menu(6.0));
    assert!(scene.service.row().is_none());
}

#[test]
fn moving_rotating_or_deleting_the_marker_stops_the_build() {
    type Change = fn(&mut BuildScene);
    let changes: [(&str, Change); 3] = [
        ("move", |scene| scene.markers.apply_hud(HudAction::Move(Axis::X, 1), 1, &[]).unwrap()),
        ("rotate", |scene| scene.markers.apply_hud(HudAction::Rotate(1), 1, &[]).unwrap()),
        ("delete", |scene| {
            let id = scene.markers.active().unwrap().id;
            scene.markers.delete(id).unwrap();
        }),
    ];
    for (name, change) in changes {
        let mut scene = BuildScene::new();
        scene.running();
        change(&mut scene);

        let sent = scene.tick(0.25);

        assert_eq!(scene.phase(), Some(BuildPhase::Stopped(BuildStopReason::MarkerChanged)), "{name}");
        assert_eq!(sent.last().map(String::as_str), Some(RESTORE), "{name}");
    }
}

#[test]
fn deleting_the_structure_stops_the_build() {
    let mut scene = BuildScene::new();
    scene.running();
    scene.library = LibraryService::new(Library::new(), LibraryConfig::default());

    let sent = scene.tick(0.25);

    assert_eq!(scene.phase(), Some(BuildPhase::Stopped(BuildStopReason::StructureDeleted)));
    assert_eq!(sent.last().map(String::as_str), Some(RESTORE));
}

#[test]
fn a_panel_that_has_not_started_closes_when_a_condition_is_lost() {
    let mut scene = BuildScene::new();
    scene.open_ready(BuildSettings::default());
    scene.service.set_conditions(BuildConditions { cheats: false, ..BuildScene::ALLOWED });

    let sent = scene.tick(0.25);

    assert!(scene.service.row().is_none());
    assert!(sent.is_empty());
}

#[test]
fn a_panel_closes_when_the_marker_moves_before_the_start() {
    let mut scene = BuildScene::new();
    scene.open_ready(BuildSettings::default());
    scene.markers.apply_hud(HudAction::Move(Axis::Z, 1), 1, &[]).unwrap();

    scene.tick(0.25);

    assert!(scene.service.row().is_none());
}

#[test]
fn the_service_locks_the_marker_only_while_a_build_runs() {
    let mut scene = BuildScene::new();
    assert!(!scene.service.locks_marker());
    scene.open_ready(BuildSettings::default());
    assert!(!scene.service.locks_marker());

    scene.apply(BuildAction::Start);
    assert!(scene.service.locks_marker());
    scene.tick(0.25);
    assert!(scene.service.locks_marker());

    scene.apply(BuildAction::Pause);
    assert!(scene.service.locks_marker());

    scene.apply(BuildAction::Stop);
    assert!(!scene.service.locks_marker());
    assert_eq!(scene.phase(), Some(BuildPhase::Stopped(BuildStopReason::Operator)));
    assert_eq!(scene.drain().last().map(String::as_str), Some(RESTORE));
}

#[test]
fn a_running_build_cannot_be_opened_again_and_cancel_leaves_it_running() {
    let mut scene = BuildScene::new();
    scene.running();

    scene.apply(BuildAction::Open(StructureId(1)));
    scene.apply(BuildAction::Cancel);

    assert_eq!(scene.phase(), Some(BuildPhase::Building));
}

#[test]
fn cancel_closes_a_finished_build() {
    let mut scene = BuildScene::new();
    scene.running();
    scene.apply(BuildAction::Stop);
    assert!(scene.service.row().is_some());

    scene.apply(BuildAction::Cancel);

    assert!(scene.service.row().is_none());
}

#[test]
fn the_up_key_pauses_and_resumes_and_the_down_key_needs_two_presses() {
    let mut scene = BuildScene::new();
    assert!(!scene.service.hud_key(HudKey::Up, 1.0));
    scene.running();

    assert!(scene.service.hud_key(HudKey::Up, 2.0));
    assert_eq!(scene.phase(), Some(BuildPhase::Paused));
    assert!(scene.service.hud_key(HudKey::Up, 3.0));
    assert_eq!(scene.phase(), Some(BuildPhase::Building));

    assert!(scene.service.hud_key(HudKey::Down, 4.0));
    assert!(scene.service.row().unwrap().confirm_stop);
    assert_eq!(scene.phase(), Some(BuildPhase::Building));
    assert!(scene.service.hud_key(HudKey::Down, 5.0));
    assert_eq!(scene.phase(), Some(BuildPhase::Stopped(BuildStopReason::Operator)));
    assert!(!scene.service.hud_key(HudKey::Left, 6.0));
}

#[test]
fn a_send_failure_stops_the_build_and_queues_the_restore() {
    let mut scene = BuildScene::new();
    scene.running();

    scene.service.send_failed();

    assert_eq!(scene.phase(), Some(BuildPhase::Stopped(BuildStopReason::SendFailed)));
    assert_eq!(scene.drain(), vec![RESTORE.to_string()]);
}

#[test]
fn settings_are_kept_per_structure_clamped_and_read_again_on_open() {
    let mut scene = BuildScene::new();
    let wild = BuildSettings { blocks_per_second: 99, clear_water: false, clear_lava: true };
    scene.apply(BuildAction::SetSettings(StructureId(1), wild));
    assert_eq!(scene.service.book().get(StructureId(1)).blocks_per_second, BuildSettings::MAX_RATE);

    scene.apply(BuildAction::Open(StructureId(1)));
    let row = scene.service.row().unwrap();
    assert_eq!(
        row.settings,
        BuildSettings { blocks_per_second: BuildSettings::MAX_RATE, clear_water: false, clear_lava: true }
    );

    // Changes while the panel is open show in the row.
    scene.apply(BuildAction::SetSettings(StructureId(1), BuildSettings { blocks_per_second: 8, ..wild }));
    assert_eq!(scene.service.row().unwrap().settings.blocks_per_second, 8);
}

#[test]
fn settings_of_a_deleted_structure_are_dropped() {
    let mut scene = BuildScene::new();
    scene.apply(BuildAction::SetSettings(StructureId(1), BuildSettings::default()));
    assert_eq!(scene.service.book().len(), 1);

    scene.service.structure_deleted(StructureId(1));

    assert!(scene.service.book().is_empty());
}

#[test]
fn a_build_that_places_every_block_is_done_with_nothing_left() {
    let mut scene = BuildScene::new();
    scene.running();

    scene.tick(10.0);

    assert_eq!(scene.phase(), Some(BuildPhase::Done));
    let row = scene.service.row().unwrap();
    assert_eq!((row.placed, row.total, row.eta_seconds), (8, 8, 0));
}

#[test]
fn a_dimension_change_stops_the_build_and_puts_the_game_rule_back() {
    let mut scene = BuildScene::new();
    scene.running();

    let sent = scene.tick_with(&Frames::in_dimension(BuildFixture::WORLD, -1, 5.0));

    assert_eq!(scene.phase(), Some(BuildPhase::Stopped(BuildStopReason::DimensionChanged)));
    assert_eq!(sent.last().map(String::as_str), Some(RESTORE));
}

#[test]
fn another_world_is_leaving_the_world_and_sends_no_command() {
    let mut scene = BuildScene::new();
    scene.running();

    let sent = scene.tick_with(&Frames::in_world("W2", 5.0));

    assert_eq!(scene.phase(), Some(BuildPhase::Stopped(BuildStopReason::LeftWorld)));
    assert!(sent.is_empty());
}

#[test]
fn an_unreadable_structure_closes_the_panel() {
    let mut scene = BuildScene::new();
    scene.apply(BuildAction::Open(StructureId(1)));

    scene.service.structure_unreadable();

    assert!(scene.service.row().is_none());
}

#[test]
fn a_rotated_structure_is_placed_and_counted_by_the_worker_before_the_panel_is_ready() {
    let piece = Pieces::layer_at(&["###", "###"], [0, 64, 0]);
    let build = BuildFixture::build(piece, Rotation::R90);
    let mut scene = BuildScene::with_build(build, [3, 1, 2]);
    scene.markers.apply_hud(HudAction::Rotate(1), 1, &[]).unwrap();
    scene.apply(BuildAction::Open(StructureId(1)));

    // The first tick only hands the structure to the worker: no palette request yet.
    scene.tick(0.0);
    assert!(scene.service.take_probe().is_none());
    assert_eq!(scene.phase(), Some(BuildPhase::Probing));

    let request = scene.probe_request();
    scene.service.apply_probe(&BuildFixture::exact_entries(request.entries.len()));
    // The count is a job too: the panel is not ready until it is back.
    assert_eq!(scene.phase(), Some(BuildPhase::Probing));
    scene.settle();
    let row = scene.service.row().unwrap();
    assert_eq!((row.phase, row.total), (BuildPhase::Ready, 6));
}

#[test]
fn an_ended_build_leaves_the_hud_after_ten_seconds_but_stays_in_the_row() {
    let mut scene = BuildScene::new();
    scene.running();
    scene.apply(BuildAction::Stop);
    scene.tick(0.0);
    let now = |scene: &BuildScene| scene.status().time_seconds;
    assert!(scene.service.hud_row(now(&scene)).is_some(), "the card shows at the end");

    scene.tick(9.0);
    assert!(scene.service.hud_row(now(&scene)).is_some(), "9 s after the end");

    scene.tick(1.0);
    assert!(scene.service.hud_row(now(&scene)).is_none(), "10 s after the end");
    assert!(scene.service.row().is_some(), "the panel row keeps the ended state");
}

#[test]
fn the_first_hud_key_after_the_end_removes_the_card_and_is_not_used() {
    let mut scene = BuildScene::new();
    scene.running();
    assert!(scene.service.hud_key(HudKey::Down, 5.0));
    assert!(scene.service.hud_key(HudKey::Down, 5.5));
    assert_eq!(scene.phase(), Some(BuildPhase::Stopped(BuildStopReason::Operator)));
    assert!(scene.service.hud_row(6.0).is_some(), "the key that stopped the build does not remove the card");

    assert!(!scene.service.hud_key(HudKey::Up, 6.0), "the key goes on to the marker keys");

    assert!(scene.service.hud_row(6.0).is_none());
    assert!(scene.service.row().is_some());
}

#[test]
fn a_new_build_shows_its_card_again() {
    let mut scene = BuildScene::new();
    scene.running();
    scene.apply(BuildAction::Stop);
    scene.tick(0.0);
    assert!(!scene.service.hud_key(HudKey::Up, 20.0));
    assert!(scene.service.hud_row(20.0).is_none());

    scene.running();
    let now = scene.status().time_seconds;
    assert!(scene.service.hud_row(now).is_some());
}

#[test]
fn a_hanging_lantern_is_placed_after_the_ceiling_block_above_it() {
    // Layer 0 holds the lantern under the ceiling stone of layer 1, and a floor block.
    let piece = Pieces::layers_at(&[&["h#"], &["##"]], [0, 64, 0]);
    let mut scene = BuildScene::with_build(BuildFixture::build(piece, Rotation::R0), [2, 2, 1]);
    scene.open_ready(BuildSettings { blocks_per_second: 60, ..BuildSettings::default() });
    scene.apply(BuildAction::Start);
    let mut sent = Vec::new();
    for _ in 0..24 {
        sent.extend(scene.tick(0.25));
    }
    let at = |position: &str| sent.iter().position(|command| command.starts_with(&format!("/setblock {position} ")));
    let lantern = at("0 64 0").expect("the lantern is placed");
    let ceiling = at("0 65 0").expect("the ceiling is placed");
    assert!(lantern > ceiling, "{sent:?}");
    assert_eq!(scene.phase(), Some(BuildPhase::Done));
}

#[test]
fn the_hud_row_is_made_once_for_each_change_and_follows_the_row() {
    let mut scene = BuildScene::new();
    scene.running();
    let now = scene.status().time_seconds;
    let first = scene.service.hud_row(now).expect("a card row");
    let again = scene.service.hud_row(now).expect("a card row");
    assert!(Arc::ptr_eq(&first, &again), "no new row while nothing changed");

    // The pace places blocks, so the row changes and a new row is made.
    scene.tick(1.0);
    let now = scene.status().time_seconds;
    let next = scene.service.hud_row(now).expect("a card row");
    assert!(!Arc::ptr_eq(&first, &next));
    assert!(next.placed > first.placed);
    assert_eq!(*next, scene.service.row().unwrap());
    let same = scene.service.hud_row(now).unwrap();
    assert!(Arc::ptr_eq(&next, &same));
}

#[test]
fn the_restore_is_owed_while_the_build_runs_and_cleared_once_it_is_sent() {
    let mut scene = BuildScene::new();
    assert!(!scene.service.book().owes("W1"));
    scene.running();
    assert!(scene.service.book().owes("W1"));
    assert!(scene.service.is_dirty());

    scene.apply(BuildAction::Stop);
    assert!(scene.service.book().owes("W1"), "owed until the command leaves the outbox");
    assert_eq!(scene.drain(), vec![RESTORE.to_string()]);
    assert!(!scene.service.book().owes("W1"));
}

#[test]
fn a_restore_that_failed_to_send_is_owed_again() {
    let mut scene = BuildScene::new();
    scene.running();
    scene.apply(BuildAction::Stop);
    assert_eq!(scene.drain(), vec![RESTORE.to_string()]);

    scene.service.send_failed();

    assert!(scene.service.book().owes("W1"));
    // The next tick in the world sends it again.
    assert_eq!(scene.tick(0.25), vec![RESTORE.to_string()]);
    assert!(!scene.service.book().owes("W1"));
}

#[test]
fn leaving_the_world_mid_build_then_entering_it_again_sends_the_restore_once() {
    let mut scene = BuildScene::new();
    scene.running();
    scene.tick(1.0);
    let left = scene.tick_with(&Frames::main_menu(5.0));
    assert!(left.is_empty());
    scene.tick_with(&Frames::main_menu(6.0));
    assert!(scene.service.book().owes("W1"), "the world is owed the restore while the player is out");

    let back = scene.tick_with(&Frames::in_world("W1", 7.0));
    let after = scene.tick_with(&Frames::in_world("W1", 8.0));

    assert_eq!(back, vec![RESTORE.to_string()]);
    assert!(after.is_empty(), "the restore is sent once");
    assert!(!scene.service.book().owes("W1"));
}

#[test]
fn another_world_gets_no_restore_and_the_owed_world_keeps_its_record() {
    let mut scene = BuildScene::new();
    scene.running();
    scene.tick_with(&Frames::main_menu(5.0));
    scene.tick_with(&Frames::main_menu(6.0));

    let other = scene.tick_with(&Frames::in_world("W2", 7.0));
    let again = scene.tick_with(&Frames::in_world("W2", 8.0));

    assert!(other.is_empty() && again.is_empty());
    assert!(scene.service.book().owes("W1"));
    assert_eq!(scene.tick_with(&Frames::in_world("W1", 9.0)), vec![RESTORE.to_string()]);
}

#[test]
fn a_world_that_does_not_allow_commands_waits_for_them_before_the_restore() {
    let cases = [
        BuildConditions { local: false, ..BuildScene::ALLOWED },
        BuildConditions { cheats: false, ..BuildScene::ALLOWED },
        BuildConditions { operator: false, ..BuildScene::ALLOWED },
    ];
    for conditions in cases {
        let mut scene = BuildScene::new();
        scene.running();
        scene.tick_with(&Frames::main_menu(5.0));
        scene.tick_with(&Frames::main_menu(6.0));
        scene.service.set_conditions(conditions);

        assert!(scene.tick_with(&Frames::in_world("W1", 7.0)).is_empty(), "{conditions:?}");
        assert!(scene.service.book().owes("W1"));
    }

    // Creative is not needed for the restore.
    let mut scene = BuildScene::new();
    scene.running();
    scene.tick_with(&Frames::main_menu(5.0));
    scene.tick_with(&Frames::main_menu(6.0));
    scene.service.set_conditions(BuildConditions { creative: false, ..BuildScene::ALLOWED });
    assert_eq!(scene.tick_with(&Frames::in_world("W1", 7.0)), vec![RESTORE.to_string()]);
}

#[test]
fn a_running_build_in_the_owed_world_is_not_given_a_second_restore() {
    let mut scene = BuildScene::new();
    scene.running();
    scene.tick_with(&Frames::main_menu(5.0));
    scene.tick_with(&Frames::main_menu(6.0));
    scene.tick_with(&Frames::in_world("W1", 7.0));
    scene.drain();

    scene.time = 8.0;
    scene.running();
    let sent = scene.tick(1.0);

    assert!(!sent.iter().any(|command| command == RESTORE), "{sent:?}");
    assert!(scene.service.book().owes("W1"));
}
