use std::cell::OnceCell;
use std::ffi::c_void;

use amml_game::{BlockLayout, BlockPos, ClientInstance, GameEntries, RawObject};

use crate::game::{BlockCalls, BlockName, BlockRegistry, ClientWorld, NbtBlock, PlayerView};
use crate::services::WorldFolder;
use crate::traits::{BlockGrid, BuildFacts, CommandSink, PaletteGame, WorldSource};
use crate::types::{BuildCalls, ConnectionInfo, NbtLookup};

// `PlayerPermissionLevel Player::getPlayerPermissionLevel() const`: the level in al
type PermissionFn = unsafe extern "C" fn(*mut c_void) -> u8;

// `LevelChunk* BlockSource::getChunk(int x, int z) const`: null for a chunk that is not loaded
type ChunkFn = unsafe extern "C" fn(*mut c_void, i32, i32) -> *const c_void;

/// The running game as the build executor reads and calls it, for one tick: the conditions, the palette lookups, the
/// block reads of the kinds scan and the command packet. It holds nothing between ticks. Game thread.
pub struct GameBuildWorld<'a> {
    client: ClientInstance<'a>,
    world: ClientWorld<'a>,
    calls: BuildCalls,
    nbt: NbtBlock,
    blocks: BlockCalls,
    // the client's block source; read on the first block read of the tick
    region: OnceCell<*mut c_void>,
}

impl<'a> GameBuildWorld<'a> {
    /// `None` for a null client.
    ///
    /// # Safety
    /// `client` must be null or the live `ClientInstance` of the game build `entries` and `calls` were resolved for,
    /// live while the value is used.
    #[allow(unsafe_code)]
    pub unsafe fn new(client: *mut c_void, entries: &'a GameEntries, calls: &BuildCalls) -> Option<Self> {
        // SAFETY: forwarded to the caller
        let world = unsafe { ClientWorld::from_raw(client, entries, WorldFolder::global()) }?;
        // SAFETY: forwarded to the caller
        let client = unsafe { ClientInstance::from_raw(client, entries) }?;
        Some(Self {
            client,
            world,
            calls: *calls,
            nbt: NbtBlock::new(BlockRegistry::new(&calls.ghost), &calls.ghost),
            blocks: BlockCalls::new(&calls.ghost),
            region: OnceCell::new(),
        })
    }

    fn region(&self) -> *mut c_void {
        *self.region.get_or_init(|| self.client.region().map_or(std::ptr::null_mut(), |region| region.as_ptr()))
    }

    fn player(&self) -> Option<*mut c_void> {
        self.client.local_player().map(|player| player.as_ptr())
    }
}

impl BuildFacts for GameBuildWorld<'_> {
    fn in_world(&self) -> bool {
        self.client.is_world_active() && self.player().is_some()
    }

    fn level(&self) -> Option<usize> {
        self.world.level()
    }

    fn connection_type(&self) -> Option<i16> {
        self.world.connection().map(|info: ConnectionInfo| info.connection_type)
    }

    fn in_realm(&self) -> bool {
        self.world.in_realm()
    }

    #[allow(unsafe_code)]
    fn game_type(&self) -> Option<i32> {
        let player = self.player()?;
        // SAFETY: the client's live local player of the build the calls were resolved for
        let view = unsafe { PlayerView::of_player(player, &self.calls.uses, &self.calls.players) }?;
        view.game_type()
    }

    #[allow(unsafe_code)]
    fn permission_level(&self) -> Option<u8> {
        if self.calls.permission_level == 0 {
            return None;
        }
        let player = self.player()?;
        // SAFETY: the entry is `Player::getPlayerPermissionLevel`, a plain function with the player in rcx and the
        // level in al
        let call: PermissionFn = unsafe { std::mem::transmute(self.calls.permission_level) };
        // SAFETY: the live local player of a loaded world, which has its abilities; the executor asks only after
        // `in_world`
        Some(unsafe { call(player) })
    }

    fn commands_flag(&self) -> Option<u8> {
        self.world.level_byte(self.calls.commands_enabled)
    }
}

impl PaletteGame for GameBuildWorld<'_> {
    fn lookup(&self, nbt: &[u8]) -> NbtLookup {
        self.nbt.lookup(nbt)
    }

    // the block is an opaque handle that `lookup` gave this tick; the game only turns it
    #[allow(unsafe_code, clippy::not_unsafe_ptr_arg_deref)]
    fn turn(&self, block: *const c_void, quarters: u8) -> *const c_void {
        // SAFETY: a block the lookup of this world gave
        unsafe { self.blocks.turn(block, quarters) }
    }
}

impl BlockGrid for GameBuildWorld<'_> {
    #[allow(unsafe_code)]
    fn chunk_loaded(&self, chunk_x: i32, chunk_z: i32) -> bool {
        let region = self.region();
        if region.is_null() {
            return false;
        }
        // SAFETY: the live block source of the client, whose vtable holds getChunk at the entry's slot
        let Some(function) = (unsafe { RawObject::slot(region, self.calls.get_chunk_slot) }) else { return false };
        // SAFETY: the slot holds `BlockSource::getChunk(int, int)`
        let get_chunk: ChunkFn = unsafe { std::mem::transmute(function) };
        // SAFETY: the live block source
        !unsafe { get_chunk(region, chunk_x, chunk_z) }.is_null()
    }

    #[allow(unsafe_code)]
    fn kind_at(&self, cell: [i32; 3]) -> Option<usize> {
        let region = self.region();
        if region.is_null() {
            return None;
        }
        let [x, y, z] = cell;
        // SAFETY: the live block source of the client
        let block = unsafe { self.blocks.world_block(region, BlockPos { x, y, z }) };
        if block.is_null() {
            return None;
        }
        // SAFETY: a live block of the world, which the mirror lays out up to its block type
        let block_type = unsafe { (&raw const (*block.cast::<BlockLayout>()).block_type).read_unaligned() };
        (!block_type.is_null()).then_some(block_type as usize)
    }

    #[allow(unsafe_code)]
    fn kind_name(&self, kind: usize) -> Option<String> {
        // SAFETY: a block type `kind_at` gave this tick, live while the world is, with its name at the entry's offset
        unsafe { BlockName::of(kind as *const c_void, self.calls.block_type_full_name) }
    }
}

impl CommandSink for GameBuildWorld<'_> {
    #[allow(unsafe_code)]
    fn send(&mut self, text: &str) -> bool {
        // SAFETY: the live client of the running update
        match unsafe { self.calls.command.send(self.client.as_ptr(), text) } {
            Ok(()) => true,
            Err(error) => {
                curia::warn!("a build command was not sent", { error: error.to_string() });
                false
            }
        }
    }
}
