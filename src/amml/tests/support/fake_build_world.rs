use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::ffi::c_void;

use alaydriem_stratum::traits::{BlockGrid, BuildFacts, CommandSink, PaletteGame};
use alaydriem_stratum::types::NbtLookup;

/// A game the build executor and its parts read, with fixed answers and counters.
pub struct FakeBuildWorld {
    pub in_world: bool,
    pub level: Option<usize>,
    pub connection_reads: Cell<u32>,
    pub connection: Option<i16>,
    pub realm: bool,
    pub game_type: Option<i32>,
    pub permission: Option<u8>,
    pub commands: Option<u8>,
    pub permission_calls: Cell<u32>,
    pub lookups: HashMap<Vec<u8>, NbtLookup>,
    pub lookup_calls: Cell<u32>,
    pub turns: HashMap<(usize, u8), usize>,
    pub unloaded_chunks: HashSet<(i32, i32)>,
    pub cells: HashMap<[i32; 3], usize>,
    pub names: HashMap<usize, String>,
    pub cell_reads: Cell<usize>,
    pub sent: Vec<String>,
    pub fail_send_at: Option<usize>,
}

impl FakeBuildWorld {
    /// A local creative world where the player is an operator and commands work.
    pub fn ready() -> Self {
        Self {
            in_world: true,
            level: Some(1),
            connection_reads: Cell::new(0),
            connection: Some(0),
            realm: false,
            game_type: Some(1),
            permission: Some(2),
            commands: Some(1),
            permission_calls: Cell::new(0),
            lookups: HashMap::new(),
            lookup_calls: Cell::new(0),
            turns: HashMap::new(),
            unloaded_chunks: HashSet::new(),
            cells: HashMap::new(),
            names: HashMap::new(),
            cell_reads: Cell::new(0),
            sent: Vec::new(),
            fail_send_at: None,
        }
    }

    /// A block pointer that is not null and tells blocks apart.
    pub fn block(id: usize) -> *const c_void {
        (0x1000 + id * 0x10) as *const c_void
    }

    /// Fills the box `min` to `max` (both inclusive) with the kind `kind`.
    pub fn fill(&mut self, min: [i32; 3], max: [i32; 3], kind: usize) {
        for x in min[0]..=max[0] {
            for y in min[1]..=max[1] {
                for z in min[2]..=max[2] {
                    self.cells.insert([x, y, z], kind);
                }
            }
        }
    }
}

impl BuildFacts for FakeBuildWorld {
    fn in_world(&self) -> bool {
        self.in_world
    }

    fn level(&self) -> Option<usize> {
        self.level
    }

    fn connection_type(&self) -> Option<i16> {
        self.connection_reads.set(self.connection_reads.get() + 1);
        self.connection
    }

    fn in_realm(&self) -> bool {
        self.realm
    }

    fn game_type(&self) -> Option<i32> {
        self.game_type
    }

    fn permission_level(&self) -> Option<u8> {
        self.permission_calls.set(self.permission_calls.get() + 1);
        self.permission
    }

    fn commands_flag(&self) -> Option<u8> {
        self.commands
    }
}

impl PaletteGame for FakeBuildWorld {
    fn lookup(&self, nbt: &[u8]) -> NbtLookup {
        self.lookup_calls.set(self.lookup_calls.get() + 1);
        self.lookups.get(nbt).copied().unwrap_or(NbtLookup::Unknown)
    }

    fn turn(&self, block: *const c_void, quarters: u8) -> *const c_void {
        match self.turns.get(&(block as usize, quarters)) {
            Some(turned) => *turned as *const c_void,
            None => block,
        }
    }
}

impl BlockGrid for FakeBuildWorld {
    fn chunk_loaded(&self, chunk_x: i32, chunk_z: i32) -> bool {
        !self.unloaded_chunks.contains(&(chunk_x, chunk_z))
    }

    fn kind_at(&self, cell: [i32; 3]) -> Option<usize> {
        self.cell_reads.set(self.cell_reads.get() + 1);
        self.cells.get(&cell).copied()
    }

    fn kind_name(&self, kind: usize) -> Option<String> {
        self.names.get(&kind).cloned()
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
