use std::ffi::c_void;

use stratum_core::services::NbtReader;
use stratum_core::structs::nbt::{NbtLimits, NbtTag};

use crate::game::{BlockRegistry, StringByteInputLayout};
use crate::types::{GhostCalls, NbtLookup};

// std::unique_ptr<CompoundTag> NbtIo::read(IDataInput&): a static function, the hidden result pointer first. On
// 26.51 the body writes one pointer, and gives an empty tag (never null) when the bytes do not read
type ReadFn = unsafe extern "C" fn(*mut *mut c_void, *mut StringByteInputLayout) -> *mut *mut c_void;
// std::pair<NBTState, Block const*> BlockSerializationUtils::tryGetBlockFromNBT(CompoundTag const&,
// NbtToBlockCache*): a static function that returns its 16 bytes through the hidden pointer
type TryGetFn = unsafe extern "C" fn(*mut [u64; 2], *const c_void, *mut c_void) -> *mut [u64; 2];
// CompoundTag::~CompoundTag(), the destructor that does not free the object
type TagDtorFn = unsafe extern "C" fn(*mut c_void);

/// Looks up palette entries by their NBT bytes, as the game's structure loader does: the bytes are read into a
/// `CompoundTag` with `NbtIo::read`, looked up by name and states, then by name with the default states.
pub struct NbtBlock {
    registry: BlockRegistry,
    calls: GhostCalls,
}

impl NbtBlock {
    /// `BlockSerializationUtils::NBTState::Success`; the state is a 4-byte int at the start of the pair.
    const STATE_SUCCESS: u32 = 2;
    /// sizeof(CompoundTag): the vtable and the `std::map` (head and size), as `NbtIo::read` allocates it.
    const TAG_BYTES: usize = 0x18;
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
        // SAFETY: the entries are the addresses of the three functions with the signatures above
        let (read, try_get, tag_dtor): (ReadFn, TryGetFn, TagDtorFn) = unsafe {
            (
                std::mem::transmute::<usize, ReadFn>(self.calls.nbt_read),
                std::mem::transmute::<usize, TryGetFn>(self.calls.try_get_block_from_nbt),
                std::mem::transmute::<usize, TagDtorFn>(self.calls.compound_tag_dtor),
            )
        };
        let mut input = StringByteInputLayout::new(self.calls.string_byte_input_vtable as *const c_void, nbt);
        let mut tag: *mut c_void = std::ptr::null_mut();
        // SAFETY: an input with the game's StringByteInput vtable over bytes that live for the call, and a
        // pointer-sized result buffer for the unique_ptr
        unsafe { read(&mut tag, &mut input) };
        if tag.is_null() {
            return std::ptr::null();
        }
        let mut pair = [0u64; 2];
        // SAFETY: a live tag the read just made, a 16-byte result buffer and no lookup cache, as LeviLamina passes it
        unsafe { try_get(&mut pair, tag, std::ptr::null_mut()) };
        // SAFETY: the tag is the one `NbtIo::read` allocated with the game's operator new for TAG_BYTES; it is
        // destroyed and freed here once, as its unique_ptr would, and not used again
        unsafe {
            tag_dtor(tag);
            (self.calls.heap_delete)(tag.cast(), Self::TAG_BYTES);
        }
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
