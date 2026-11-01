use alaydriem_stratum::game::OwnedItemStack;
use amml_game::ItemStackLayout;

use crate::support::FakePacket;
use crate::support::fake_packet::{STACK_COPY_CALLS, STACK_DTOR_CALLS, STACK_DTOR_FLAGS};

#[test]
#[allow(unsafe_code)]
fn a_copied_stack_is_destroyed_once_through_the_full_destructor_without_a_free() {
    let table = FakePacket::stack_vtable();
    let calls = FakePacket::player_calls(table.as_ptr());
    let source = std::mem::MaybeUninit::<ItemStackLayout>::zeroed();

    // SAFETY: the fake copy constructor reads nothing from the source
    let copy = unsafe { OwnedItemStack::copy_of(source.as_ptr(), &calls) };

    assert_eq!(FakePacket::count(&STACK_COPY_CALLS), 1);
    assert_eq!(FakePacket::count(&STACK_DTOR_CALLS), 0);
    drop(copy);
    assert_eq!(FakePacket::count(&STACK_DTOR_CALLS), 1);
    // the scalar deleting destructor with no delete flag: the destructor runs and the memory stays
    assert_eq!(STACK_DTOR_FLAGS.load(std::sync::atomic::Ordering::SeqCst), 0);
}

#[test]
#[allow(unsafe_code)]
fn a_new_stack_holds_the_item_stack_vtable_when_the_game_builds_only_the_base() {
    let table = FakePacket::stack_vtable();
    let mut calls = FakePacket::player_calls(table.as_ptr());
    calls.item_stack_empty_ctor = FakePacket::base_stack_ctor as *const () as usize;

    // SAFETY: the fake constructor writes only the first qword of the 152-byte stack
    let stack = unsafe { OwnedItemStack::new(&calls) };

    // SAFETY: the stack is 152 bytes; its first qword is the vtable
    let first = unsafe { stack.as_ptr().cast::<usize>().read() };
    assert_eq!(first, table.as_ptr() as usize);
}

#[test]
#[allow(unsafe_code)]
fn the_after_step_changes_nothing_after_a_2651_constructor() {
    let table = FakePacket::stack_vtable();
    let mut calls = FakePacket::player_calls(table.as_ptr());
    FakePacket::set_full_stack_vtable(table.as_ptr() as usize);
    calls.item_stack_empty_ctor = FakePacket::full_stack_ctor as *const () as usize;

    // SAFETY: the fake constructor writes inside the 152-byte stack and keeps a copy of what it wrote
    let stack = unsafe { OwnedItemStack::new(&calls) };

    // SAFETY: the stack is 152 bytes
    let bytes = unsafe { std::slice::from_raw_parts(stack.as_ptr().cast::<u8>(), size_of::<ItemStackLayout>()) };
    assert_eq!(bytes, FakePacket::full_stack_bytes().as_slice());
}

#[test]
#[allow(unsafe_code)]
fn the_after_step_writes_the_2660_inline_bytes_after_a_base_constructor() {
    let table = FakePacket::stack_vtable();
    let mut calls = FakePacket::player_calls(table.as_ptr());
    calls.item_stack_empty_ctor = FakePacket::dirty_base_stack_ctor as *const () as usize;

    // SAFETY: the fake constructor writes inside the 152-byte stack
    let stack = unsafe { OwnedItemStack::new(&calls) };

    // SAFETY: the stack is 152 bytes
    let bytes = unsafe { std::slice::from_raw_parts(stack.as_ptr().cast::<u8>(), size_of::<ItemStackLayout>()) };
    assert_eq!(bytes[0..8], (table.as_ptr() as usize).to_le_bytes());
    assert_eq!(bytes[0x80..0x84], [0, 0, 0, 0]);
    assert_eq!(bytes[0x90], 0);
    // the bytes the 26.60 code does not store stay as the base constructor left them
    assert_eq!(bytes[0x84..0x88], [0xEE; 4]);
}
