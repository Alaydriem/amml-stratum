use alaydriem_stratum::build::ConditionsReader;
use amml_api::{ConnectionKind, GameType, PermissionLevel};
use stratum_core::structs::build::BuildConditions;

use crate::support::FakeBuildWorld;

#[test]
fn a_local_creative_operator_with_cheats_may_build() {
    let world = FakeBuildWorld::ready();

    let conditions = ConditionsReader::new(true).read(&world);

    assert_eq!(conditions, BuildConditions { local: true, cheats: true, operator: true, creative: true });
}

#[test]
fn conditions_are_false_when_the_feature_is_off() {
    let world = FakeBuildWorld::ready();

    let conditions = ConditionsReader::new(false).read(&world);

    assert_eq!(conditions, BuildConditions::default());
    assert_eq!(world.permission_calls.get(), 0, "no game call while the feature is off");
}

#[test]
fn remote_or_realm_is_not_local() {
    let mut server = FakeBuildWorld::ready();
    server.connection = Some(ConnectionKind::Remote);
    let mut realm = FakeBuildWorld::ready();
    realm.realm = true;
    let mut unknown = FakeBuildWorld::ready();
    unknown.connection = None;

    assert!(!ConditionsReader::new(true).read(&server).local);
    assert!(!ConditionsReader::new(true).read(&realm).local);
    assert!(!ConditionsReader::new(true).read(&unknown).local);
}

#[test]
fn creative_is_the_creative_game_type_only() {
    let mut reader = ConditionsReader::new(true);
    let mut world = FakeBuildWorld::ready();

    world.game_type = Some(GameType::Survival);
    assert!(!reader.read(&world).creative);
    world.game_type = Some(GameType::Adventure);
    assert!(!reader.read(&world).creative);
    world.game_type = None;
    assert!(!reader.read(&world).creative);
    world.game_type = Some(GameType::Creative);
    assert!(reader.read(&world).creative);
}

#[test]
fn operator_is_the_operator_level_or_above() {
    let mut reader = ConditionsReader::new(true);
    let mut world = FakeBuildWorld::ready();

    world.permission = Some(PermissionLevel::Member);
    assert!(!reader.read(&world).operator);
    world.permission = None;
    assert!(!reader.read(&world).operator);
    world.permission = Some(PermissionLevel::Operator);
    assert!(reader.read(&world).operator);
    world.permission = Some(PermissionLevel::Custom);
    assert!(reader.read(&world).operator);
}

#[test]
fn cheats_follow_the_commands_setting() {
    let mut reader = ConditionsReader::new(true);
    let mut world = FakeBuildWorld::ready();

    world.commands = Some(false);
    assert!(!reader.read(&world).cheats);
    world.commands = None;
    assert!(!reader.read(&world).cheats);
    world.commands = Some(true);
    assert!(reader.read(&world).cheats);
}

#[test]
fn the_permission_is_never_read_without_a_player_in_a_world() {
    let mut world = FakeBuildWorld::ready();
    world.in_world = false;

    let conditions = ConditionsReader::new(true).read(&world);

    assert_eq!(conditions, BuildConditions::default());
    assert_eq!(world.permission_calls.get(), 0);
}

#[test]
fn the_connection_is_read_once_while_the_world_stays() {
    let world = FakeBuildWorld::ready();
    let mut reader = ConditionsReader::new(true);

    for _ in 0..5 {
        assert!(reader.read(&world).local);
    }

    assert_eq!(world.connection_reads.get(), 1);
}

#[test]
fn the_connection_is_read_again_for_another_world() {
    let mut world = FakeBuildWorld::ready();
    let mut reader = ConditionsReader::new(true);
    assert!(reader.read(&world).local);

    world.level = Some(2);
    world.connection = Some(ConnectionKind::Remote);

    assert!(!reader.read(&world).local);
    assert_eq!(world.connection_reads.get(), 2);
}

#[test]
fn a_connection_the_game_has_not_set_is_read_again_next_time() {
    let mut world = FakeBuildWorld::ready();
    world.connection = None;
    let mut reader = ConditionsReader::new(true);
    assert!(!reader.read(&world).local);

    world.connection = Some(ConnectionKind::Local);

    assert!(reader.read(&world).local);
    assert_eq!(world.connection_reads.get(), 2);
}

#[test]
fn a_realm_is_not_local_even_when_its_connection_was_read_as_local() {
    let mut world = FakeBuildWorld::ready();
    let mut reader = ConditionsReader::new(true);
    assert!(reader.read(&world).local);

    world.realm = true;

    assert!(!reader.read(&world).local);
}

#[test]
fn the_connection_is_read_again_after_leaving_a_world() {
    let mut world = FakeBuildWorld::ready();
    let mut reader = ConditionsReader::new(true);
    assert!(reader.read(&world).local);

    world.in_world = false;
    reader.read(&world);
    world.in_world = true;
    world.connection = Some(ConnectionKind::Remote);

    assert!(!reader.read(&world).local);
}

#[test]
fn a_missing_level_forgets_the_connection() {
    let mut world = FakeBuildWorld::ready();
    let mut reader = ConditionsReader::new(true);
    assert!(reader.read(&world).local);

    world.level = None;
    reader.read(&world);
    world.level = Some(1);
    world.connection = Some(ConnectionKind::Remote);

    assert!(!reader.read(&world).local);
}

#[test]
fn an_undefined_connection_is_read_again_next_time() {
    let mut world = FakeBuildWorld::ready();
    // the loader gives the game's undefined connection type as Other
    world.connection = Some(ConnectionKind::Other);
    let mut reader = ConditionsReader::new(true);
    assert!(!reader.read(&world).local);

    world.connection = Some(ConnectionKind::Local);

    assert!(reader.read(&world).local);
    assert_eq!(world.connection_reads.get(), 2);
}
