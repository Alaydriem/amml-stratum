use std::ffi::c_void;
use std::mem::MaybeUninit;

use amml_game::{BlockPos, RawObject};
use amml_msvc::SharedPtr;
use stratum_core::structs::ghost::ChestPair;

use crate::game::GameCompoundTag;
use crate::types::{BlockActorCalls, ChestPairCalls};

// shared_ptr<BlockActor> VanillaBlockActorFactory::createBlockActor(BlockPos const&, BlockType const&): a static
// function, the hidden result pointer first, the buffer back in rax
type CreateFn = unsafe extern "C" fn(*mut SharedPtr<c_void>, *const BlockPos, *const c_void) -> *mut SharedPtr<c_void>;
// bool BlockActor::isType(BlockActorType) const: the actor, then the type byte; the answer in al
type IsTypeFn = unsafe extern "C" fn(*const c_void, u8) -> u8;
// shared_ptr<BlockActor> BlockActor::loadStatic(BlockType const&, BlockPos const&, ILevel&, CompoundTag const&,
// DataLoadHelper&): a static function, the hidden result pointer first, the buffer back in rax
type LoadStaticFn = unsafe extern "C" fn(
    *mut SharedPtr<c_void>,
    *const c_void,
    *const BlockPos,
    *mut c_void,
    *const c_void,
    *mut c_void,
) -> *mut SharedPtr<c_void>;

/// A block actor of the game, made by the game's factory or loaded from block entity NBT, held through the
/// `std::shared_ptr<BlockActor>` the game made. The use it holds is given back once, through the control block, when
/// it is dropped.
pub struct GameBlockActor {
    actor: SharedPtr<c_void>,
}

impl GameBlockActor {
    /// Bit 1 of the chest's pair flags byte: set when the pair runs along Z. `ChestRenderer::render` reads it with
    /// `movzx eax,byte ptr [r14+2F8h]; and al,2; shr al,1` and compares the paired position on that axis.
    pub const PAIR_ALONG_Z: u8 = 0b10;

    /// `BlockActorType::Chest`. `ChestBlockActor::isType` answers true for it in every chest-family actor, and the
    /// factory makes chest, trapped chest and copper chest actors with it.
    pub const CHEST_TYPE: u8 = 2;

    /// Whether the game's own `isType(Chest)` says this actor is a `ChestBlockActor`: the check the chest renderer
    /// makes before it reads the chest fields. False for no actor.
    #[allow(unsafe_code)]
    pub fn is_chest(&self, calls: &ChestPairCalls) -> bool {
        let actor = self.as_ptr();
        if actor.is_null() {
            return false;
        }
        // SAFETY: a live actor this wrapper holds a use of; its vtable holds `isType` at the entry's slot
        let Some(function) = (unsafe { RawObject::slot(actor, calls.is_type_slot) }) else {
            return false;
        };
        // SAFETY: the slot holds `bool isType(BlockActorType) const`, called on its own actor
        let is_type = unsafe { std::mem::transmute::<*const c_void, IsTypeFn>(function) };
        // SAFETY: as above; the call reads the actor only
        unsafe { is_type(actor, Self::CHEST_TYPE) != 0 }
    }

    /// Makes this chest the lead of a large chest whose other half is at `pair` from it. The partner pointer is the
    /// actor's own address, which the chest renderer only compares with null and which lives exactly as long as the
    /// actor; the paired position is the actor's own `mPosition` moved by the pair offset; the pair axis bit is set
    /// for a Z pair and cleared for an X pair, and the other flag bits stay. Call it once, right after the make.
    ///
    /// # Safety
    /// `calls` must hold the entries of the running build, and this actor must be a `ChestBlockActor`
    /// ([`Self::is_chest`] true), so it spans at least `chest_large_paired_position + 12` bytes. Call it on the render
    /// thread.
    #[allow(unsafe_code)]
    pub unsafe fn set_large_chest(&mut self, calls: &ChestPairCalls, pair: ChestPair) {
        let actor = self.as_ptr();
        if actor.is_null() {
            return;
        }
        let (Ok(position), Ok(flags), Ok(paired), Ok(paired_position)) = (
            usize::try_from(calls.actor_position),
            usize::try_from(calls.chest_pair_flags),
            usize::try_from(calls.chest_large_paired),
            usize::try_from(calls.chest_large_paired_position),
        ) else {
            return;
        };
        let base = actor.cast::<u8>();
        // SAFETY: a live actor this wrapper holds a use of, with `mPosition` at the entry's offset
        let own = unsafe { base.add(position).cast::<BlockPos>().read_unaligned() };
        let [dx, dy, dz] = pair.offset();
        let partner = BlockPos { x: own.x.wrapping_add(dx), y: own.y.wrapping_add(dy), z: own.z.wrapping_add(dz) };
        // SAFETY: the caller guarantees a live ChestBlockActor, at least `chest_large_paired_position + 12` bytes
        // (0x31c on 26.63, of 0x380), so the flags byte, the partner pointer and the partner position at the entries'
        // offsets are inside it; the render thread is the only one that touches the actor
        unsafe {
            let byte = base.add(flags);
            let old = byte.read();
            let new = if dx == 0 { old | Self::PAIR_ALONG_Z } else { old & !Self::PAIR_ALONG_Z };
            byte.write(new);
            base.add(paired).cast::<*mut c_void>().write_unaligned(actor);
            base.add(paired_position).cast::<BlockPos>().write_unaligned(partner);
        }
    }

    /// The actor of `block_type` at `pos`. With NBT bytes the tag is read with `NbtIo::read`, the actor is loaded
    /// with `BlockActor::loadStatic` and a `DefaultDataLoadHelper` on the stack, and the tag is destroyed and freed
    /// once; with no bytes (or empty bytes) the factory makes a default actor. `None` for a null block type, for a
    /// load with a null level, when no tag is read, or when the game gives an empty pointer.
    ///
    /// # Safety
    /// `calls` must hold the entries of the running build; `block_type` must be null or a live `BlockType`; `level`
    /// must be null or the live level of the client (the `ClientInstance::getLevel` result). Call it on the render
    /// thread.
    #[allow(unsafe_code)]
    pub unsafe fn load(
        calls: &BlockActorCalls,
        block_type: *const c_void,
        pos: BlockPos,
        nbt: Option<&[u8]>,
        level: *mut c_void,
    ) -> Option<Self> {
        if block_type.is_null() {
            return None;
        }
        let actor = match nbt {
            Some(bytes) if !bytes.is_empty() => {
                if level.is_null() {
                    return None;
                }
                // SAFETY: the caller guarantees the entries, a live block type and a live level
                unsafe { Self::load_static(calls, block_type, pos, bytes, level) }?
            }
            // SAFETY: the caller guarantees the entries and a live block type
            _ => unsafe { Self::create(calls, block_type, pos) },
        };
        let made = Self { actor };
        if made.as_ptr().is_null() { None } else { Some(made) }
    }

    /// The actor object, as the dispatcher takes it; null for none.
    pub fn as_ptr(&self) -> *mut c_void {
        self.actor.get().map_or(std::ptr::null_mut(), |actor| std::ptr::from_ref(actor).cast_mut())
    }

    /// The actor's `IVanillaRenderBlockActorComponent` from `_getRenderComponent`; null for an actor with none.
    #[allow(unsafe_code)]
    pub fn render_component(&self, calls: &BlockActorCalls) -> *mut c_void {
        let actor = self.as_ptr();
        if actor.is_null() {
            return std::ptr::null_mut();
        }
        // SAFETY: a live actor this wrapper holds a use of; its vtable holds `_getRenderComponent` at the entry's slot
        let Some(function) = (unsafe { RawObject::slot(actor, calls.render_component_slot) }) else {
            return std::ptr::null_mut();
        };
        // SAFETY: the slot holds `IVanillaRenderBlockActorComponent* _getRenderComponent()`, called on its actor
        unsafe { RawObject::call_pointer(function, actor) }.unwrap_or(std::ptr::null_mut())
    }

    // a default actor from the factory; an empty pointer for a type with no actor
    #[allow(unsafe_code)]
    unsafe fn create(calls: &BlockActorCalls, block_type: *const c_void, pos: BlockPos) -> SharedPtr<c_void> {
        // SAFETY: the entry is the address of `VanillaBlockActorFactory::createBlockActor`
        let create: CreateFn = unsafe { std::mem::transmute::<usize, CreateFn>(calls.create_block_actor) };
        let mut out = MaybeUninit::<SharedPtr<c_void>>::zeroed();
        // SAFETY: a buffer for a shared_ptr, a position that lives for the call and the caller's live block type
        unsafe { create(out.as_mut_ptr(), &pos, block_type) };
        // SAFETY: the factory wrote a shared_ptr, or left the zeroed, empty one
        unsafe { out.assume_init() }
    }

    // the actor loaded from `nbt`; None when no tag is read. The tag is destroyed and freed once before this returns
    #[allow(unsafe_code)]
    unsafe fn load_static(
        calls: &BlockActorCalls,
        block_type: *const c_void,
        pos: BlockPos,
        nbt: &[u8],
        level: *mut c_void,
    ) -> Option<SharedPtr<c_void>> {
        // SAFETY: the entry is the address of the function with the signature above
        let load = unsafe { std::mem::transmute::<usize, LoadStaticFn>(calls.load_static) };
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
        }?;
        // the DefaultDataLoadHelper is only its vptr, as the game builds it on the stack
        let mut helper: usize = calls.data_load_helper_vtable;
        let mut out = MaybeUninit::<SharedPtr<c_void>>::zeroed();
        // SAFETY: a buffer for a shared_ptr, the caller's live block type and level, a position, a tag and a helper
        // that live for the call
        unsafe {
            load(out.as_mut_ptr(), block_type, &pos, level, tag.as_ptr(), (&raw mut helper).cast::<c_void>());
        }
        drop(tag);
        // SAFETY: the load wrote a shared_ptr, or left the zeroed, empty one
        Some(unsafe { out.assume_init() })
    }
}

impl Drop for GameBlockActor {
    #[allow(unsafe_code)]
    fn drop(&mut self) {
        // SAFETY: the game gave the pointer with one use, which this wrapper owns and gives back once
        unsafe { self.actor.release() };
    }
}
