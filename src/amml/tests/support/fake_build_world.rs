use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};

use alaydriem_stratum::traits::{BlockGrid, BuildFacts, CommandSink, PaletteGame};
use alaydriem_stratum::types::PaletteLookup;
use amml_api::{ApiError, BlockHandle, BlockPos, ConnectionKind, GameType, KindsInBox, PermissionLevel};

/// A game the build executor and its parts read, with fixed answers and counters.
pub struct FakeBuildWorld {
    pub in_world: bool,
    pub level: Option<u64>,
    pub connection_reads: Cell<u32>,
    pub connection: Option<ConnectionKind>,
    pub realm: bool,
    pub game_type: Option<GameType>,
    pub permission: Option<PermissionLevel>,
    pub commands: Option<bool>,
    pub permission_calls: Cell<u32>,
    pub lookups: HashMap<Vec<u8>, PaletteLookup>,
    pub lookup_calls: Cell<u32>,
    pub turns: HashMap<(BlockHandle, u8), BlockHandle>,
    pub unloaded_chunks: HashSet<(i32, i32)>,
    pub cells: HashMap<[i32; 3], String>,
    // the boxes `kinds_in` was asked for, as (min, max)
    pub boxes: RefCell<Vec<([i32; 3], [i32; 3])>>,
    pub sent: Vec<String>,
    pub fail_send_at: Option<usize>,
}

impl FakeBuildWorld {
    pub const AIR: &'static str = "minecraft:air";

    /// A local creative world where the player is an operator and commands work.
    pub fn ready() -> Self {
        Self {
            in_world: true,
            level: Some(1),
            connection_reads: Cell::new(0),
            connection: Some(ConnectionKind::Local),
            realm: false,
            game_type: Some(GameType::Creative),
            permission: Some(PermissionLevel::Operator),
            commands: Some(true),
            permission_calls: Cell::new(0),
            lookups: HashMap::new(),
            lookup_calls: Cell::new(0),
            turns: HashMap::new(),
            unloaded_chunks: HashSet::new(),
            cells: HashMap::new(),
            boxes: RefCell::new(Vec::new()),
            sent: Vec::new(),
            fail_send_at: None,
        }
    }

    /// A block handle that tells blocks apart.
    pub fn block(id: u64) -> BlockHandle {
        BlockHandle::for_test(0x1000 + id)
    }

    /// Fills the box `min` to `max` (both inclusive) with the block `name`.
    pub fn fill(&mut self, min: [i32; 3], max: [i32; 3], name: &str) {
        for x in min[0]..=max[0] {
            for y in min[1]..=max[1] {
                for z in min[2]..=max[2] {
                    self.cells.insert([x, y, z], name.to_string());
                }
            }
        }
    }
}

impl BuildFacts for FakeBuildWorld {
    fn in_world(&self) -> bool {
        self.in_world
    }

    fn level(&self) -> Option<u64> {
        self.level
    }

    fn connection_kind(&self) -> Option<ConnectionKind> {
        self.connection_reads.set(self.connection_reads.get() + 1);
        self.connection
    }

    fn in_realm(&self) -> bool {
        self.realm
    }

    fn game_type(&self) -> Option<GameType> {
        self.game_type
    }

    fn permission_level(&self) -> Option<PermissionLevel> {
        self.permission_calls.set(self.permission_calls.get() + 1);
        self.permission
    }

    fn commands_enabled(&self) -> Option<bool> {
        self.commands
    }
}

impl PaletteGame for FakeBuildWorld {
    fn lookup(&self, nbt: &[u8]) -> PaletteLookup {
        self.lookup_calls.set(self.lookup_calls.get() + 1);
        self.lookups.get(nbt).copied().unwrap_or(PaletteLookup::Unknown)
    }

    fn turn(&self, block: BlockHandle, quarters: u8) -> Option<BlockHandle> {
        Some(self.turns.get(&(block, quarters)).copied().unwrap_or(block))
    }
}

// the loader's rules for one box: every chunk loaded, at most `KINDS_CELL_CAP` cells, at most `max_kinds` kinds, the
// kinds in the order x, z, y
impl BlockGrid for FakeBuildWorld {
    fn kinds_in(&self, min: BlockPos, max: BlockPos, max_kinds: u32) -> Result<KindsInBox, ApiError> {
        self.boxes.borrow_mut().push(([min.x, min.y, min.z], [max.x, max.y, max.z]));
        let cells = (i64::from(max.x - min.x) + 1) * (i64::from(max.y - min.y) + 1) * (i64::from(max.z - min.z) + 1);
        if min.x > max.x || min.y > max.y || min.z > max.z || cells > 65_536 {
            return Err(ApiError::Refused);
        }
        for chunk_x in (min.x >> 4)..=(max.x >> 4) {
            for chunk_z in (min.z >> 4)..=(max.z >> 4) {
                if self.unloaded_chunks.contains(&(chunk_x, chunk_z)) {
                    return Ok(KindsInBox::NotLoaded);
                }
            }
        }
        let mut kinds: Vec<String> = Vec::new();
        for x in min.x..=max.x {
            for z in min.z..=max.z {
                for y in min.y..=max.y {
                    let name = self.cells.get(&[x, y, z]).map_or(Self::AIR, String::as_str);
                    if !kinds.iter().any(|kind| kind == name) {
                        if kinds.len() as u32 >= max_kinds {
                            return Ok(KindsInBox::TooMany);
                        }
                        kinds.push(name.to_string());
                    }
                }
            }
        }
        Ok(KindsInBox::Kinds(kinds))
    }
}

impl CommandSink for FakeBuildWorld {
    fn send(&mut self, text: &str) -> bool {
        if self.fail_send_at == Some(self.sent.len()) {
            return false;
        }
        self.sent.push(text.to_string());
        true
    }
}
