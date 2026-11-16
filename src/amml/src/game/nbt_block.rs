use std::ffi::c_void;

use stratum_core::services::NbtReader;
use stratum_core::structs::nbt::{NbtLimits, NbtTag};

use crate::game::{BlockRegistry, GameCompoundTag};
use crate::types::{GhostCalls, NbtLookup};

// std::pair<NBTState, Block const*> BlockSerializationUtils::tryGetBlockFromNBT(CompoundTag const&,
// NbtToBlockCache*): a static function that returns its 16 bytes through the hidden pointer
type TryGetFn = unsafe extern "C" fn(*mut [u64; 2], *const c_void, *mut c_void) -> *mut [u64; 2];

/// Looks up palette entries by their NBT bytes, as the game's structure loader does: the bytes are read into a
/// `CompoundTag` with `NbtIo::read`, looked up by name and states, then by name with the default states.
pub struct NbtBlock {
    registry: BlockRegistry,
    calls: GhostCalls,
}

impl NbtBlock {
    /// `BlockSerializationUtils::NBTState::Success`; the state is a 4-byte int at the start of the pair.
    const STATE_SUCCESS: u32 = 2;
    /// The limits of one palette entry: the entry compound, its states compound and their values.
    const ENTRY_LIMITS: NbtLimits =
        NbtLimits { max_depth: 8, max_list_len: 4096, max_total_bytes: 1024 * 1024, max_decoded_int_list: 4096 };

    pub fn new(registry: BlockRegistry, calls: &GhostCalls) -> Self {
        Self { registry, calls: *calls }
    }

    /// The block of one palette entry's little-endian NBT bytes (`{ name, states, version }`). Every tag the game
    /// reads is destroyed and freed once before this returns.
    pub fn lookup(&self, nbt: &[u8]) -> NbtLookup {
        if nbt.is_empty() {
            return NbtLookup::Unknown;
        }
        let exact = self.exact(nbt);
        if !exact.is_null() {
            return NbtLookup::Exact(exact);
        }
        let Some(name) = Self::name_of(nbt) else { return NbtLookup::Unknown };
        let block = self.registry.by_name(&name);
        if block.is_null() { NbtLookup::Unknown } else { NbtLookup::Default(block) }
    }

    // the block the game gives for the bytes' name and states; null when it gives none or reads no tag
    #[allow(unsafe_code)]
    fn exact(&self, nbt: &[u8]) -> *const c_void {
        // SAFETY: the entry is the address of the function with the signature above
        let try_get = unsafe { std::mem::transmute::<usize, TryGetFn>(self.calls.try_get_block_from_nbt) };
        let calls = &self.calls;
        // SAFETY: the entries are `NbtIo::read`, the `StringByteInput` vtable, `CompoundTag::~CompoundTag` and the
        // game heap of the running build
        let tag = unsafe {
            GameCompoundTag::read(
                calls.nbt_read,
                calls.string_byte_input_vtable,
                calls.compound_tag_dtor,
                calls.heap_delete,
                nbt,
            )
        };
        let Some(tag) = tag else { return std::ptr::null() };
        let mut pair = [0u64; 2];
        // SAFETY: a live tag the read just made, a 16-byte result buffer and no lookup cache, as LeviLamina passes it
        unsafe { try_get(&mut pair, tag.as_ptr(), std::ptr::null_mut()) };
        drop(tag);
        // the game writes only the low 4 bytes of the first qword, the int, and leaves the padding as it was
        let state = pair[0] as u32;
        if state == Self::STATE_SUCCESS { pair[1] as *const c_void } else { std::ptr::null() }
    }

    // the entry's `name` string; `None` when the bytes do not read or hold no name
    fn name_of(nbt: &[u8]) -> Option<String> {
        let (_, root) = NbtReader::read(nbt, Self::ENTRY_LIMITS).ok()?;
        match root.get("name")? {
            NbtTag::String(name) => Some(name.clone()),
            _ => None,
        }
    }
}
