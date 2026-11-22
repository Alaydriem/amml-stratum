use std::collections::HashMap;
use std::path::PathBuf;

use alaydriem_stratum::StratumMod;
use amm_client::test_support::FakeTable;
use amml_api::amml_abi::{
    AmmlDirKind, AppFocusLost, BlockChanged, BlockEntityPass, BuildBlock, ClientTick, DimensionChanging,
    InventoryRequestDone, MouseInput, ServerJoinStarting, UsePressed, WindowFocus, WorldJoined, WorldLeft,
    WorldRendered,
};
use amml_api::testing::{FakeClient, FakeGame, FakeWorld};
use amml_api::{Area, BlockHandle, Connection, ConnectionKind, GameType, PermissionLevel, Rotation, Vec3, WorldInfo};
use tempfile::TempDir;

// stratum enabled on a fake game that serves every row of its `USES`, with the fake manager's tables provided and
// ready, and a client in a local creative world with cheats on, the player an operator at 10, 64, -20 in the overworld
pub struct StratumRig {
    pub game: FakeGame,
    // the data folder the loader gives the mod, kept for the life of the rig
    pub _data: TempDir,
}

impl StratumRig {
    pub const STONE: u64 = 1;
    pub const AIR: u64 = 2;
    pub const UNKNOWN: u64 = 3;
    pub const FOLDER: &'static str = "My World";

    pub fn new() -> Self {
        let game = FakeTable::game();
        game.offer::<ClientTick>();
        game.offer::<WorldJoined>();
        game.offer::<WorldLeft>();
        game.offer::<DimensionChanging>();
        game.offer::<WorldRendered>();
        game.offer::<BlockChanged>();
        game.offer::<BlockEntityPass>();
        game.offer::<UsePressed>();
        game.offer::<BuildBlock>();
        game.offer::<InventoryRequestDone>();
        game.offer::<MouseInput>();
        game.offer::<WindowFocus>();
        game.offer::<AppFocusLost>();
        game.offer::<ServerJoinStarting>();
        for area in Self::areas() {
            game.offer_area(area);
        }
        let data = TempDir::new().unwrap();
        game.set_dir(AmmlDirKind::DATA, data.path().to_str().unwrap());
        FakeTable::set_config("");
        // the fake has one mod: the manager's tables are provided before stratum's lifecycle, so the link finds them
        // at its start
        FakeTable::provide(&game.ctx_for::<StratumMod>()).unwrap();
        game.load::<StratumMod>().unwrap();
        game.enable::<StratumMod>().unwrap();
        game.open_gate();
        game.set_epoch(1);
        game.set_client(Self::client());
        Self { game, _data: data }
    }

    pub fn areas() -> Vec<Area> {
        vec![
            Area::INPUT_LEASE,
            Area::CLIENT_READ,
            Area::CLIENT_WORLD_INFO,
            Area::ACTOR_READ.at(4),
            Area::OPTIONS_VIEW_DISTANCE,
            Area::RENDER_WORLD_CANVAS.at(3),
            Area::RENDER_BLOCK_MESHER,
            Area::WORLD_BLOCK_STATES,
            Area::RENDER_WATER_TINT,
            Area::RENDER_BLOCK_ENTITIES,
            Area::RENDER_CHEST_PAIR,
            Area::RENDER_SKULL_POSE,
            Area::WORLD_BLOCKS,
            Area::PLAYER_BUILD,
            Area::PLAYER_PICK_RANGE,
            Area::PLAYER_HIT_RESULT,
            Area::HUD_SELECT_HOTBAR,
            Area::INVENTORY_READ.at(3),
            Area::INVENTORY_REQUEST.at(2),
            Area::CLIENT_SEND_COMMAND,
            Area::ACTOR_GAME_TYPE,
            Area::ACTOR_PERMISSION,
            Area::WORLD_KINDS_IN_BOX,
        ]
    }

    // the blocks of the fake world: stone, air and the unknown block by name
    pub fn world() -> FakeWorld {
        let mut states = HashMap::new();
        for (name, id) in
            [("minecraft:stone", Self::STONE), ("minecraft:air", Self::AIR), ("minecraft:unknown", Self::UNKNOWN)]
        {
            states.insert((name.to_string(), Vec::new()), BlockHandle::for_test(id));
        }
        let mut block_names = HashMap::new();
        block_names.insert(BlockHandle::for_test(Self::STONE), "minecraft:stone".to_string());
        block_names.insert(BlockHandle::for_test(Self::AIR), "minecraft:air".to_string());
        FakeWorld { states, block_names, ..FakeWorld::default() }
    }

    pub fn client() -> FakeClient {
        FakeClient {
            local_player: true,
            world_active: true,
            position: Some(Vec3 { x: 10.5, y: 64.0, z: -19.5 }),
            head_position: Some(Vec3 { x: 10.5, y: 65.8, z: -19.5 }),
            rotation: Some(Rotation::default()),
            view_vector: Some(Vec3 { x: 0.0, y: -1.0, z: 0.0 }),
            dimension: Some(0),
            game_type: Some(GameType::Creative),
            permission_level: Some(PermissionLevel::Operator),
            build_delay: Some(150),
            max_pick_range: Some(12.0),
            world: Some(Self::world()),
            world_info: Some(WorldInfo {
                storage_folder: Some(PathBuf::from(format!(r"C:\worlds\{}", Self::FOLDER))),
                level_name: Some("Level".to_string()),
                seed: Some(7),
                realm: Some(false),
                commands_enabled: Some(true),
                connection: Some(Connection { kind: ConnectionKind::Local, host: String::new(), port: 0 }),
            }),
            ..FakeClient::default()
        }
    }

    // the manager's tables published: the link connects
    pub fn ready(&self) {
        FakeTable::ready(&self.game);
    }

    pub fn tick(&self) {
        self.game.raise_with_client(&mut ClientTick { header: FakeGame::header::<ClientTick>() });
    }

    // the lines of the mod's log that hold `fragment`
    pub fn lines(&self, fragment: &str) -> Vec<String> {
        self.game.logs().into_iter().filter(|line| line.contains(fragment)).collect()
    }
}

impl Drop for StratumRig {
    fn drop(&mut self) {
        self.game.disable::<StratumMod>();
    }
}
