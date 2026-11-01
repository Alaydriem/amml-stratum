use alaydriem_stratum::detours::db_storage_shutdown::StorageShutdown;
use alaydriem_stratum::services::{WorldFolder, WorldKeyReader};

use crate::support::FakeWorld;

const OLD_STORAGE: usize = 0x5000;
const OTHER_STORAGE: usize = 0x6000;

#[test]
fn a_storage_that_is_not_done_yet_keeps_its_folder() {
    let folder = WorldFolder::new();
    folder.set(OLD_STORAGE, Some("Old World".to_string()));

    StorageShutdown::run(false, OLD_STORAGE, &folder);

    assert_eq!(folder.current().as_deref(), Some("Old World"));
}

#[test]
fn the_shutdown_of_another_storage_keeps_the_folder() {
    let folder = WorldFolder::new();
    folder.set(OLD_STORAGE, Some("Old World".to_string()));

    StorageShutdown::run(true, OTHER_STORAGE, &folder);

    assert_eq!(folder.current().as_deref(), Some("Old World"));
}

#[test]
fn a_failed_world_load_then_a_server_whose_first_read_is_empty_is_never_named_by_the_old_folder() {
    let folder = WorldFolder::new();
    folder.set(OLD_STORAGE, Some("Old World".to_string()));
    // the world never loads: its storage shuts down with no client level seen
    StorageShutdown::run(true, OLD_STORAGE, &folder);
    let server = FakeWorld::server("play.example.net", 19132);
    *server.connection.borrow_mut() = None;
    *server.folder.borrow_mut() = folder.current();
    let mut reader = WorldKeyReader::new();
    let mut key = String::new();

    server.read(&mut reader, &mut key, 1);
    assert_eq!(key, "");
    *server.connection.borrow_mut() = Some(FakeWorld::connection(1, "203.0.113.7", "play.example.net", 19132));
    server.read(&mut reader, &mut key, WorldKeyReader::SERVER_STABLE_READS);

    assert_eq!(key, "server:play.example.net:19132");
}
