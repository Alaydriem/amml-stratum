use alaydriem_stratum::detours::db_storage_shutdown::StorageShutdown;
use alaydriem_stratum::services::{FrameStatusReader, WorldFolder, WorldKeyReader};
use alaydriem_stratum::types::PlayerSpot;

use crate::support::FakeWorld;

const OLD_STORAGE: usize = 0x5000;

#[test]
fn with_no_local_player_the_status_is_in_world_with_an_empty_world() {
    let world = FakeWorld::local("My World");
    world.player.set(None);
    world.menu.set(true);
    let mut reader = FrameStatusReader::new();

    let status = reader.read(&world, 4.5).clone();

    assert!(status.in_world);
    assert!(status.world.is_empty());
    assert!(!status.has_player);
    assert!(!status.menu_open, "nothing past the player is read");
    assert_eq!(status.time_seconds, 4.5);
}

#[test]
fn the_key_read_while_no_player_shows_once_the_player_is_there() {
    let world = FakeWorld::local("My World");
    world.player.set(None);
    let mut reader = FrameStatusReader::new();
    reader.read(&world, 1.0);
    world.player.set(Some(PlayerSpot { dimension: 1, feet: [-3, 70, 8] }));

    let status = reader.read(&world, 2.0).clone();

    assert_eq!(status.world, "local:My World");
    assert!(status.has_player);
    assert_eq!((status.dimension, status.feet), (1, [-3, 70, 8]));
}

#[test]
fn in_a_world_with_a_player_every_field_is_read() {
    let world = FakeWorld::local("My World");
    world.menu.set(true);
    world.view.set(16);
    let mut reader = FrameStatusReader::new();

    let status = reader.read(&world, 3.0).clone();

    assert!(status.in_world && status.has_player && status.menu_open);
    assert_eq!(status.world, "local:My World");
    assert_eq!((status.dimension, status.feet), (0, [10, 64, -20]));
    assert_eq!(status.render_distance, 16);
    assert_eq!(status.time_seconds, 3.0);
}

#[test]
fn a_view_distance_below_one_reads_as_zero() {
    let world = FakeWorld::local("My World");
    world.view.set(-1);
    let mut reader = FrameStatusReader::new();

    assert_eq!(reader.read(&world, 1.0).render_distance, 0);
}

#[test]
fn out_of_a_world_only_the_time_is_set_and_the_key_is_forgotten() {
    let world = FakeWorld::local("My World");
    let mut reader = FrameStatusReader::new();
    reader.read(&world, 1.0);
    world.active.set(false);

    let status = reader.read(&world, 2.0).clone();
    world.active.set(true);
    *world.folder.borrow_mut() = Some("Other World".to_string());
    let back = reader.read(&world, 3.0).clone();

    assert!(!status.in_world && !status.has_player);
    assert!(status.world.is_empty());
    assert_eq!(status.time_seconds, 2.0);
    assert_eq!(back.world, "local:Other World");
}

#[test]
fn a_tick_out_of_the_world_keeps_the_local_key_and_it_is_logged_once() {
    let world = FakeWorld::local("My World");
    let mut reader = FrameStatusReader::new();
    reader.read(&world, 1.0);
    world.active.set(false);
    reader.read(&world, 2.0);
    world.active.set(true);

    let back = reader.read(&world, 3.0).clone();

    assert_eq!(back.world, "local:My World");
    assert_eq!(reader.world_key().logged(), 1);
}

#[test]
fn a_storage_shutdown_out_of_the_world_then_a_server_with_an_empty_first_read_never_names_the_old_folder() {
    let folder = WorldFolder::new();
    folder.set(OLD_STORAGE, Some("Old World".to_string()));
    let world = FakeWorld::local("Old World");
    let mut reader = FrameStatusReader::new();
    reader.read(&world, 1.0);
    world.active.set(false);
    reader.read(&world, 2.0);
    StorageShutdown::run(true, OLD_STORAGE, &folder);
    world.active.set(true);
    world.level.set(Some(0x2000));
    *world.connection.borrow_mut() = None;
    *world.folder.borrow_mut() = folder.current();

    let mut keys = vec![reader.read(&world, 3.0).world.clone()];
    *world.connection.borrow_mut() = Some(FakeWorld::connection(1, "203.0.113.7", "play.example.net", 19132));
    for tick in 0..WorldKeyReader::SERVER_STABLE_READS {
        keys.push(reader.read(&world, 4.0 + f64::from(tick)).world.clone());
    }

    assert!(keys.iter().all(|key| !key.starts_with("local:")), "{keys:?}");
    assert_eq!(keys.last().map(String::as_str), Some("server:play.example.net:19132"));
}
