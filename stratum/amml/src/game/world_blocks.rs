use std::ffi::c_void;

use amml_game::{BlockLayout, BlockPos, ItemInstanceLayout, RawObject};

use crate::game::OwnedItemInstance;
use crate::types::UseCalls;

// `Block const& BlockSource::getBlock(BlockPos const&) const`: the reference comes back in rax
type GetBlockFn = unsafe extern "C" fn(*mut c_void, *const BlockPos) -> *const c_void;

// `bool BlockType::canBeBuiltOver(Block const&, BlockSource const&, BlockPos const&) const`
type CanBeBuiltOverFn = unsafe extern "C" fn(*const c_void, *const c_void, *mut c_void, *const BlockPos) -> u8;

// `bool BlockType::isInteractiveBlock() const`
type IsInteractiveFn = unsafe extern "C" fn(*const c_void) -> u8;

// `ItemInstance BlockType::asItemInstance(Block const&, BlockActor const*) const`: this, the hidden return pointer,
// the block, the block actor
type AsItemInstanceFn = unsafe extern "C" fn(
    *const c_void,
    *mut ItemInstanceLayout,
    *const c_void,
    *const c_void,
) -> *mut ItemInstanceLayout;

/// The block questions easy place asks of a region: which block a cell holds, whether a placed block would replace
/// it, whether a block can be placed against it, and which item a block is.
#[derive(Clone, Copy)]
pub struct WorldBlocks {
    calls: UseCalls,
}

impl WorldBlocks {
    pub fn new(calls: &UseCalls) -> Self {
        Self { calls: *calls }
    }

    /// The block at `pos`; null when the entry is 0.
    ///
    /// # Safety
    /// `region` must be the live `BlockSource` of the world.
    #[allow(unsafe_code)]
    pub unsafe fn block_at(&self, region: *mut c_void, pos: &BlockPos) -> *const c_void {
        if self.calls.block_source_get_block == 0 || region.is_null() {
            return std::ptr::null();
        }
        // SAFETY: the entry is the address of getBlock with the signature of `GetBlockFn`
        let get: GetBlockFn = unsafe { std::mem::transmute(self.calls.block_source_get_block) };
        // SAFETY: the caller guarantees a live region; `pos` lives for the call
        unsafe { get(region, pos) }
    }

    /// Whether a placed block replaces the block at `pos`: `canBeBuiltOver` of its block type. False when the block
    /// or its type is null.
    ///
    /// # Safety
    /// As [`WorldBlocks::block_at`].
    #[allow(unsafe_code)]
    pub unsafe fn replaceable(&self, region: *mut c_void, pos: &BlockPos) -> bool {
        // SAFETY: forwarded to the caller
        let block = unsafe { self.block_at(region, pos) };
        // SAFETY: a block the region gave, live while the world is
        unsafe { self.built_over(block, region, pos) }
    }

    /// Whether a block can be placed against the block at `pos`: it cannot be built over and is not an interactive
    /// block (a door, a chest or a sign would open instead of taking the block).
    ///
    /// # Safety
    /// As [`WorldBlocks::block_at`].
    #[allow(unsafe_code)]
    pub unsafe fn against(&self, region: *mut c_void, pos: &BlockPos) -> bool {
        // SAFETY: forwarded to the caller
        let block = unsafe { self.block_at(region, pos) };
        // SAFETY: a block the region gave, live while the world is
        let Some(block_type) = (unsafe { Self::block_type(block) }) else { return false };
        // SAFETY: as above
        if unsafe { self.built_over(block, region, pos) } {
            return false;
        }
        // SAFETY: the live block type, whose vtable holds isInteractiveBlock at the entry's slot
        let Some(function) = (unsafe { RawObject::slot(block_type, self.calls.is_interactive_block_slot) }) else {
            return false;
        };
        // SAFETY: the slot holds isInteractiveBlock
        let call: IsInteractiveFn = unsafe { std::mem::transmute(function) };
        // SAFETY: the live block type
        unsafe { call(block_type) == 0 }
    }

    /// The item `block` is (`asItemInstance` with no block actor); None for a null block or type.
    ///
    /// # Safety
    /// `block` must be null or a live `Block` of the game.
    #[allow(unsafe_code)]
    pub unsafe fn item_of(&self, block: *const c_void) -> Option<OwnedItemInstance> {
        // SAFETY: forwarded to the caller
        let block_type = unsafe { Self::block_type(block) }?;
        // SAFETY: the live block type, whose vtable holds asItemInstance at the entry's slot
        let function = unsafe { RawObject::slot(block_type, self.calls.as_item_instance_slot) }?;
        // SAFETY: the slot holds asItemInstance with the signature of `AsItemInstanceFn`
        let call: AsItemInstanceFn = unsafe { std::mem::transmute(function) };
        // SAFETY: the callee constructs the item instance in the room it is given, destroyed by the base destructor
        unsafe {
            OwnedItemInstance::made_by(self.calls.item_stack_base_dtor, |out| {
                call(block_type, out, block, std::ptr::null());
                true
            })
        }
    }

    // canBeBuiltOver of the block's type; false for a null block or type
    #[allow(unsafe_code)]
    unsafe fn built_over(&self, block: *const c_void, region: *mut c_void, pos: &BlockPos) -> bool {
        // SAFETY: the caller passes a null or live block
        let Some(block_type) = (unsafe { Self::block_type(block) }) else { return false };
        // SAFETY: the live block type, whose vtable holds canBeBuiltOver at the entry's slot
        let Some(function) = (unsafe { RawObject::slot(block_type, self.calls.can_be_built_over_slot) }) else {
            return false;
        };
        // SAFETY: the slot holds the three-parameter canBeBuiltOver
        let call: CanBeBuiltOverFn = unsafe { std::mem::transmute(function) };
        // SAFETY: the live block type, block and region, and a position that lives for the call
        unsafe { call(block_type, block, region, pos) != 0 }
    }

    // the block's `mBlockType`; None for a null block or type
    #[allow(unsafe_code)]
    unsafe fn block_type(block: *const c_void) -> Option<*const c_void> {
        if block.is_null() {
            return None;
        }
        // SAFETY: the caller passes a live block, which the mirror lays out up to mBlockType
        let block_type = unsafe { (&raw const (*block.cast::<BlockLayout>()).block_type).read_unaligned() };
        (!block_type.is_null()).then_some(block_type.cast_const())
    }
}
