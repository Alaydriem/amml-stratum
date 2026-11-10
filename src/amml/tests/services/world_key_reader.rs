use alaydriem_stratum::services::WorldKeyReader;
use alaydriem_stratum::types::ConnectionInfo;

use crate::support::FakeWorld;

#[test]
fn a_local_world_is_named_by_its_folder_on_the_first_read() {
    let world = FakeWorld::local("My World");

    assert_eq!(world.key_after(1), "local:My World");
}

#[test]
fn a_loopback_connection_of_another_type_still_counts_as_local() {
    let world = FakeWorld::local("My World");
    *world.connection.borrow_mut() = Some(FakeWorld::connection(1, "127.0.0.1", "", 19132));

    assert_eq!(world.key_after(1), "local:My World");
}

#[test]
fn a_remote_connection_is_never_named_by_a_storage_folder() {
    let world = FakeWorld::server("play.example.net", 19132);
    *world.folder.borrow_mut() = Some("My World".to_string());

    assert_eq!(world.key_after(WorldKeyReader::SERVER_STABLE_READS), "server:play.example.net:19132");
}

#[test]
fn a_local_connection_with_no_storage_folder_falls_back_to_the_name() {
    let world = FakeWorld::new();
    *world.connection.borrow_mut() = Some(FakeWorld::connection(ConnectionInfo::LOCAL, "", "", 0));

    assert_eq!(world.key_after(WorldKeyReader::FALLBACK_AFTER_READS - 1), "");
    assert_eq!(world.key_after(WorldKeyReader::FALLBACK_AFTER_READS), "name:Level:14274599075807261974");
}

#[test]
fn a_server_key_is_taken_after_twenty_equal_reads() {
    let world = FakeWorld::server("play.example.net", 19132);

    assert_eq!(world.key_after(WorldKeyReader::SERVER_STABLE_READS - 1), "");
    assert_eq!(world.key_after(WorldKeyReader::SERVER_STABLE_READS), "server:play.example.net:19132");
}

#[test]
fn a_server_without_a_url_is_named_by_its_host_and_an_ipv6_host_is_bracketed() {
    let world = FakeWorld::server("", 19132);
    let ipv6 = FakeWorld::new();
    *ipv6.connection.borrow_mut() = Some(FakeWorld::connection(1, "2001:db8::1", "", 19133));

    assert_eq!(world.key_after(WorldKeyReader::SERVER_STABLE_READS), "server:203.0.113.7:19132");
    assert_eq!(ipv6.key_after(WorldKeyReader::SERVER_STABLE_READS), "server:[2001:db8::1]:19133");
}

#[test]
fn a_server_key_that_changes_starts_the_count_again() {
    let world = FakeWorld::server("a.example.net", 19132);
    let mut reader = WorldKeyReader::new();
    let mut key = String::new();
    world.read(&mut reader, &mut key, WorldKeyReader::SERVER_STABLE_READS - 1);
    *world.connection.borrow_mut() = Some(FakeWorld::connection(1, "", "b.example.net", 19132));

    world.read(&mut reader, &mut key, WorldKeyReader::SERVER_STABLE_READS - 1);
    assert_eq!(key, "");
    world.read(&mut reader, &mut key, 1);

    assert_eq!(key, "server:b.example.net:19132");
}

#[test]
fn a_server_key_that_never_settles_is_taken_at_the_fallback_read() {
    let world = FakeWorld::server("a.example.net", 19132);
    let mut reader = WorldKeyReader::new();
    let mut key = String::new();
    for read in 1..WorldKeyReader::FALLBACK_AFTER_READS {
        let url = if read % 2 == 0 { "a.example.net" } else { "b.example.net" };
        *world.connection.borrow_mut() = Some(FakeWorld::connection(1, "", url, 19132));
        world.read(&mut reader, &mut key, 1);
        assert_eq!(key, "", "read {read}");
    }

    // the 300th read sees the key of read 299 a second time
    world.read(&mut reader, &mut key, 1);

    assert_eq!(key, "server:b.example.net:19132");
}

#[test]
fn a_realm_is_named_by_its_level_name_on_the_first_read() {
    let world = FakeWorld::new();
    world.in_realm.set(true);
    *world.level_name.borrow_mut() = Some("Realm Name".to_string());

    assert_eq!(world.key_after(1), "realm:Realm Name");
}

#[test]
fn a_world_with_no_source_takes_the_name_key_after_300_reads() {
    let world = FakeWorld::new();

    assert_eq!(world.key_after(WorldKeyReader::FALLBACK_AFTER_READS - 1), "");
    assert_eq!(world.key_after(WorldKeyReader::FALLBACK_AFTER_READS), "name:Level:14274599075807261974");
}

#[test]
fn a_realm_whose_name_is_not_known_yet_drops_a_server_key() {
    let world = FakeWorld::server("play.example.net", 19132);
    let mut reader = WorldKeyReader::new();
    let mut key = String::new();
    world.read(&mut reader, &mut key, WorldKeyReader::SERVER_STABLE_READS);
    assert_eq!(key, "server:play.example.net:19132");
    world.in_realm.set(true);
    *world.level_name.borrow_mut() = Some(String::new());

    world.read(&mut reader, &mut key, 1);
    assert_eq!(key, "");
    *world.level_name.borrow_mut() = Some("Realm Name".to_string());
    world.read(&mut reader, &mut key, 1);

    assert_eq!(key, "realm:Realm Name");
}

#[test]
fn a_realm_that_shows_by_the_600th_read_replaces_a_server_key() {
    let world = FakeWorld::server("play.example.net", 19132);
    *world.level_name.borrow_mut() = Some("Realm Name".to_string());
    let mut reader = WorldKeyReader::new();
    let mut key = String::new();
    world.read(&mut reader, &mut key, WorldKeyReader::REALM_RECHECK_READS - 1);
    assert_eq!(key, "server:play.example.net:19132");
    world.in_realm.set(true);

    world.read(&mut reader, &mut key, 1);

    assert_eq!(key, "realm:Realm Name");
}

#[test]
fn after_600_reads_a_server_or_name_key_stays() {
    let server = FakeWorld::server("play.example.net", 19132);
    let named = FakeWorld::new();
    let (mut server_reader, mut named_reader) = (WorldKeyReader::new(), WorldKeyReader::new());
    let (mut server_key, mut named_key) = (String::new(), String::new());
    server.read(&mut server_reader, &mut server_key, WorldKeyReader::REALM_RECHECK_READS);
    named.read(&mut named_reader, &mut named_key, WorldKeyReader::REALM_RECHECK_READS);
    for world in [&server, &named] {
        world.in_realm.set(true);
        *world.level_name.borrow_mut() = Some("Realm Name".to_string());
    }

    server.read(&mut server_reader, &mut server_key, 1);
    named.read(&mut named_reader, &mut named_key, 1);

    assert_eq!(server_key, "server:play.example.net:19132");
    assert_eq!(named_key, "name:Level:14274599075807261974");
}

#[test]
fn a_local_key_stays_when_a_realm_shows_later() {
    let world = FakeWorld::local("My World");
    let mut reader = WorldKeyReader::new();
    let mut key = String::new();
    world.read(&mut reader, &mut key, 1);
    world.in_realm.set(true);

    world.read(&mut reader, &mut key, 5);

    assert_eq!(key, "local:My World");
}

#[test]
fn a_new_level_reads_the_game_again_and_no_level_clears_the_key() {
    let world = FakeWorld::local("My World");
    let mut reader = WorldKeyReader::new();
    let mut key = String::new();
    world.read(&mut reader, &mut key, 1);
    world.level.set(None);

    world.read(&mut reader, &mut key, 1);
    assert_eq!(key, "");
    world.level.set(Some(0x2000));
    *world.folder.borrow_mut() = Some("Other World".to_string());
    world.read(&mut reader, &mut key, 1);

    assert_eq!(key, "local:Other World");
}

#[test]
fn a_reset_forgets_the_world() {
    let world = FakeWorld::local("My World");
    let mut reader = WorldKeyReader::new();
    let mut key = String::new();
    world.read(&mut reader, &mut key, 1);
    *world.folder.borrow_mut() = Some("Other World".to_string());

    reader.reset();
    key.clear();
    world.read(&mut reader, &mut key, 1);

    assert_eq!(key, "local:Other World");
}

#[test]
fn an_empty_level_name_still_gives_the_name_key_as_the_game_prints_it() {
    let world = FakeWorld::new();
    *world.level_name.borrow_mut() = Some(String::new());

    assert_eq!(world.key_after(WorldKeyReader::FALLBACK_AFTER_READS), "name::14274599075807261974");
}

#[test]
fn a_level_name_that_cannot_be_read_takes_no_name_key_until_it_can() {
    let world = FakeWorld::new();
    *world.level_name.borrow_mut() = None;
    let mut reader = WorldKeyReader::new();
    let mut key = String::new();

    world.read(&mut reader, &mut key, WorldKeyReader::FALLBACK_AFTER_READS);
    assert_eq!(key, "");
    *world.level_name.borrow_mut() = Some("Level".to_string());
    world.read(&mut reader, &mut key, 1);

    assert_eq!(key, "name:Level:14274599075807261974");
}

#[test]
fn a_seed_that_cannot_be_read_takes_no_name_key_until_it_can() {
    let world = FakeWorld::new();
    world.seed.set(None);
    let mut reader = WorldKeyReader::new();
    let mut key = String::new();

    world.read(&mut reader, &mut key, WorldKeyReader::FALLBACK_AFTER_READS);
    assert_eq!(key, "");
    world.seed.set(Some(FakeWorld::SEED));
    world.read(&mut reader, &mut key, 1);

    assert_eq!(key, "name:Level:14274599075807261974");
}
