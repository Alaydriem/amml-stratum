use std::cell::{Cell, RefCell};

use alaydriem_stratum::services::WorldKeyReader;
use alaydriem_stratum::traits::{ClientSource, WorldSource};
use alaydriem_stratum::types::{ConnectionInfo, PlayerSpot};

// A game client with fixed answers, changed between reads through the cells. It starts in a world of level 0x1000,
// named "Level", with no connection info, no storage folder and a player at (10, 64, -20) in the overworld
pub struct FakeWorld {
    pub active: Cell<bool>,
    pub level: Cell<Option<usize>>,
    pub in_realm: Cell<bool>,
    pub connection: RefCell<Option<ConnectionInfo>>,
    pub folder: RefCell<Option<String>>,
    pub level_name: RefCell<Option<String>>,
    pub seed: Cell<Option<u64>>,
    pub player: Cell<Option<PlayerSpot>>,
    pub menu: Cell<bool>,
    pub view: Cell<i32>,
}

impl FakeWorld {
    // the seed of the name key tests, a negative 64-bit seed as the game stores it
    pub const SEED: u64 = (-4_172_144_997_902_289_642_i64) as u64;

    pub fn new() -> Self {
        Self {
            active: Cell::new(true),
            level: Cell::new(Some(0x1000)),
            in_realm: Cell::new(false),
            connection: RefCell::new(None),
            folder: RefCell::new(None),
            level_name: RefCell::new(Some("Level".to_string())),
            seed: Cell::new(Some(Self::SEED)),
            player: Cell::new(Some(PlayerSpot { dimension: 0, feet: [10, 64, -20] })),
            menu: Cell::new(false),
            view: Cell::new(12),
        }
    }

    pub fn connection(connection_type: i16, host: &str, url: &str, port: i32) -> ConnectionInfo {
        ConnectionInfo { connection_type, host: host.to_string(), url: url.to_string(), port }
    }

    // a local world whose storage folder is `folder`, over a local connection
    pub fn local(folder: &str) -> Self {
        let world = Self::new();
        *world.connection.borrow_mut() = Some(Self::connection(ConnectionInfo::LOCAL, "", "", 0));
        *world.folder.borrow_mut() = Some(folder.to_string());
        world
    }

    // a server joined by `url`, with no storage folder
    pub fn server(url: &str, port: i32) -> Self {
        let world = Self::new();
        *world.connection.borrow_mut() = Some(Self::connection(1, "203.0.113.7", url, port));
        world
    }

    // the key after `reads` more reads by `reader`, written into `key`
    pub fn read(&self, reader: &mut WorldKeyReader, key: &mut String, reads: u32) {
        for _ in 0..reads {
            reader.read(self, key);
        }
    }

    // the key of a fresh reader after `reads` reads
    pub fn key_after(&self, reads: u32) -> String {
        let mut reader = WorldKeyReader::new();
        let mut key = String::new();
        self.read(&mut reader, &mut key, reads);
        key
    }
}

impl WorldSource for FakeWorld {
    fn level(&self) -> Option<usize> {
        self.level.get()
    }

    fn in_realm(&self) -> bool {
        self.in_realm.get()
    }

    fn connection(&self) -> Option<ConnectionInfo> {
        self.connection.borrow().clone()
    }

    fn local_folder(&self) -> Option<String> {
        self.folder.borrow().clone()
    }

    fn level_name(&self) -> Option<String> {
        self.level_name.borrow().clone()
    }

    fn seed(&self) -> Option<u64> {
        self.seed.get()
    }
}

impl ClientSource for FakeWorld {
    fn is_world_active(&self) -> bool {
        self.active.get()
    }

    fn player(&self) -> Option<PlayerSpot> {
        self.player.get()
    }

    fn menu_open(&self) -> bool {
        self.menu.get()
    }

    fn view_distance_chunks(&self) -> i32 {
        self.view.get()
    }
}
