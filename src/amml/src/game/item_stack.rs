use std::ffi::c_void;
use std::mem::{MaybeUninit, offset_of};

use amml_game::{ItemStackLayout, RawObject};

use crate::types::PlayerCalls;

// `ItemStack::ItemStack(ItemStack const&)`: rcx the new stack, rdx the source, the new stack back in rax
type CopyCtorFn = unsafe extern "C" fn(*mut c_void, *const c_void) -> *mut c_void;

// the empty stack constructor (`ItemStack::ItemStack()` on 26.51, `ItemStackBase::ItemStackBase()` on 26.60): rcx
// the new stack, returned in rax
type CtorFn = unsafe extern "C" fn(*mut c_void) -> *mut c_void;

// `void ItemStack::reinit(Item const&, int count, int auxValue)`: this, the item, the count, the aux value
type ReinitFn = unsafe extern "C" fn(*mut c_void, *const c_void, i32, i32);

// the scalar deleting destructor in slot 0 of the ItemStack vtable: rcx the stack, edx the flags (bit 0 frees)
type DeletingDtorFn = unsafe extern "C" fn(*mut c_void, u32) -> *mut c_void;

/// An `ItemStack` the mod owns, in its own memory, made by the game's copy constructor. It is destroyed once, when it
/// is dropped, through the full `ItemStack` destructor (the vtable's scalar deleting destructor with no free), which
/// clears a net id variant that holds a request id before the `ItemStackBase` part.
pub struct OwnedItemStack {
    stack: Box<MaybeUninit<ItemStackLayout>>,
    vtable: usize,
}

impl OwnedItemStack {
    // the flags of the scalar deleting destructor that run the destructor and keep the memory
    const KEEP_MEMORY: u32 = 0;
    // the net id variant stores after the constructor, as both builds write them: the 32-bit value at +80h
    // (`mov dword ptr [x+80h],0`) and the index byte at +90h (`mov byte ptr [x+90h],0`), the empty net id
    const NET_ID_VALUE: usize = offset_of!(ItemStackLayout, net_id_variant);
    const NET_ID_INDEX: usize = offset_of!(ItemStackLayout, net_id_variant) + 0x10;

    /// An empty stack, as `ItemStack()` makes it: the game's empty stack constructor, then the stores that the 26.60
    /// callers write inline after `ItemStackBase::ItemStackBase()` (the `ItemStack` vtable at +0, a zero 32-bit net
    /// id at +80h and a zero variant index at +90h). The 26.51 `ItemStack::ItemStack()` ends with the same three
    /// stores, so on 26.51 the after-step writes the bytes that are already there.
    ///
    /// # Safety
    /// `calls` must be resolved for the running build.
    #[allow(unsafe_code)]
    pub unsafe fn new(calls: &PlayerCalls) -> Self {
        let mut stack = Box::new(MaybeUninit::<ItemStackLayout>::zeroed());
        // SAFETY: the entry is the address of the empty stack constructor
        let ctor: CtorFn = unsafe { std::mem::transmute(calls.item_stack_empty_ctor) };
        // SAFETY: 152 bytes of the mod's memory, aligned for the stack
        unsafe { ctor(stack.as_mut_ptr().cast()) };
        let bytes = stack.as_mut_ptr().cast::<u8>();
        // SAFETY: inside the 152-byte stack: its vtable pointer at +0 (the entry is the ItemStack vtable), and the
        // value and index of the net id variant
        unsafe {
            bytes.cast::<usize>().write(calls.item_stack_vtable);
            bytes.add(Self::NET_ID_VALUE).cast::<u32>().write_unaligned(0);
            bytes.add(Self::NET_ID_INDEX).write(0);
        }
        Self { stack, vtable: calls.item_stack_vtable }
    }

    /// Makes the stack `count` items of `item` with `aux`, through the `ItemStack` vtable's `reinit(Item const&, int,
    /// int)`.
    ///
    /// # Safety
    /// `item` must be a live `Item`, and `calls` the calls this stack was made with.
    #[allow(unsafe_code)]
    pub unsafe fn reinit(&mut self, item: *const c_void, count: i32, aux: i32, calls: &PlayerCalls) {
        let this = self.stack.as_mut_ptr().cast::<c_void>();
        // SAFETY: a stack the game's constructor made; its vtable is the ItemStack vtable
        let Some(function) = (unsafe { RawObject::slot(this, calls.item_stack_reinit_slot) }) else { return };
        // SAFETY: the slot holds the Item overload of reinit
        let reinit: ReinitFn = unsafe { std::mem::transmute(function) };
        // SAFETY: the live stack and the caller's live item
        unsafe { reinit(this, item, count, aux) };
    }

    /// A copy of `source`, as `ItemStack(ItemStack const&)` makes it.
    ///
    /// # Safety
    /// `source` must be a live `ItemStack` of the build `calls` was resolved for.
    #[allow(unsafe_code)]
    pub unsafe fn copy_of(source: *const ItemStackLayout, calls: &PlayerCalls) -> Self {
        let mut stack = Box::new(MaybeUninit::<ItemStackLayout>::zeroed());
        // SAFETY: the entry is the address of the copy constructor
        let copy: CopyCtorFn = unsafe { std::mem::transmute(calls.item_stack_copy_ctor) };
        // SAFETY: 152 bytes of the mod's memory, aligned for the stack, and a live source stack
        unsafe { copy(stack.as_mut_ptr().cast(), source.cast()) };
        Self { stack, vtable: calls.item_stack_vtable }
    }

    /// The stack, as the game's `setItem` takes it.
    pub fn as_ptr(&self) -> *const ItemStackLayout {
        self.stack.as_ptr()
    }

    /// The stack's members.
    pub fn layout(&self) -> &ItemStackLayout {
        // SAFETY: the copy constructor made the stack
        #[allow(unsafe_code)]
        unsafe {
            self.stack.assume_init_ref()
        }
    }
}

impl Drop for OwnedItemStack {
    fn drop(&mut self) {
        if self.vtable == 0 {
            return;
        }
        // SAFETY: the entry is the ItemStack vtable, whose slot 0 holds its scalar deleting destructor
        #[allow(unsafe_code)]
        let function = unsafe { *(self.vtable as *const usize) };
        if function == 0 {
            return;
        }
        // SAFETY: the slot holds the scalar deleting destructor
        #[allow(unsafe_code)]
        let dtor: DeletingDtorFn = unsafe { std::mem::transmute(function) };
        // SAFETY: the stack the copy constructor made, destroyed once; with no free flag the box keeps its memory
        #[allow(unsafe_code)]
        unsafe {
            dtor(self.stack.as_mut_ptr().cast(), Self::KEEP_MEMORY)
        };
    }
}
