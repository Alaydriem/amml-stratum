use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};

use alaydriem_stratum::ghost::BlockChangeListener;
use alaydriem_stratum::services::BlockChangeHub;
use alaydriem_stratum::types::BlockChange;
use amml_game::BlockPos;

use crate::support::{FakeGhost, FakeSource};

static CHANGES: AtomicUsize = AtomicUsize::new(0);
static LAST_LAYER: AtomicUsize = AtomicUsize::new(usize::MAX);

fn count(change: &BlockChange) {
    CHANGES.fetch_add(1, Ordering::SeqCst);
    LAST_LAYER.store(change.layer as usize, Ordering::SeqCst);
}

type BlockChanged = extern "C" fn(
    *mut c_void,
    *mut c_void,
    *const BlockPos,
    u32,
    *mut c_void,
    *mut c_void,
    i32,
    *const c_void,
    u8,
    *mut c_void,
);
type SourceDestroyed = extern "C" fn(*mut c_void, *mut c_void);

#[allow(unsafe_code)]
fn listener() -> BlockChangeListener {
    let hub: &'static BlockChangeHub = Box::leak(Box::new(BlockChangeHub::new(count)));
    // SAFETY: the base is a 9-slot table and the fake heap a valid heap
    unsafe { BlockChangeListener::new(FakeSource::listener_base(), FakeSource::slots(), hub, &FakeGhost::heap()) }
        .unwrap()
}

// calls a slot as the game does: through the object's own vtable
fn fire_block_changed(listener: &BlockChangeListener, source: *mut c_void, layer: u32) {
    let thunk = listener.vtable()[FakeSource::BLOCK_CHANGED as usize];
    // SAFETY: the slot holds the onBlockChanged thunk
    let thunk: BlockChanged = unsafe { std::mem::transmute(thunk) };
    let pos = BlockPos { x: 1, y: 2, z: 3 };
    let null = std::ptr::null_mut();
    thunk(listener.object(), source, &pos, layer, null, null, 3, std::ptr::null(), 0, null);
}

fn fire_source_destroyed(listener: &BlockChangeListener, source: *mut c_void) {
    let thunk = listener.vtable()[FakeSource::SOURCE_DESTROYED as usize];
    // SAFETY: the slot holds the onSourceDestroyed thunk
    let thunk: SourceDestroyed = unsafe { std::mem::transmute(thunk) };
    thunk(listener.object(), source);
}

#[test]
#[allow(unsafe_code)]
fn leaving_removes_the_listener_from_the_source_it_was_added_to() {
    let mut source = FakeSource::new();
    let listener = listener();

    // SAFETY: a live fake source
    unsafe { listener.attach(source.ptr()) };
    listener.detach();

    let added = FakeSource::added();
    assert_eq!(added, [(source.ptr() as usize, listener.object() as usize)]);
    assert_eq!(FakeSource::removed(), added);
}

#[test]
#[allow(unsafe_code)]
fn a_destroyed_source_then_a_leave_never_asks_that_source_to_remove_the_listener() {
    let mut source = FakeSource::new();
    let listener = listener();
    // SAFETY: a live fake source
    unsafe { listener.attach(source.ptr()) };

    fire_source_destroyed(&listener, source.ptr());
    listener.detach();

    assert_eq!(FakeSource::added().len(), 1);
    assert!(FakeSource::removed().is_empty());
    assert!(!listener.is_attached());
}

#[test]
#[allow(unsafe_code)]
fn a_move_to_another_region_leaves_the_old_one_first() {
    let (mut old, mut new) = (FakeSource::new(), FakeSource::new());
    let listener = listener();

    // SAFETY: live fake sources
    unsafe {
        listener.attach(old.ptr());
        listener.attach(new.ptr());
    }

    assert_eq!(FakeSource::removed(), [(old.ptr() as usize, listener.object() as usize)]);
    assert_eq!(FakeSource::added().last(), Some(&(new.ptr() as usize, listener.object() as usize)));
}

#[test]
fn each_block_change_reaches_the_hub_with_its_layer() {
    let listener = listener();

    fire_block_changed(&listener, 0x9000 as *mut c_void, 1);

    assert_eq!(CHANGES.load(Ordering::SeqCst), 1);
    assert_eq!(LAST_LAYER.load(Ordering::SeqCst), 1);
}

#[test]
#[allow(unsafe_code)]
fn a_registered_listener_is_not_freed_and_a_detached_one_is_freed_once() {
    let mut source = FakeSource::new();
    let listener = listener();
    let object = listener.object() as usize;
    // SAFETY: a live fake source
    unsafe { listener.attach(source.ptr()) };

    // SAFETY: the fake heap made the object; the fake source runs no thunk on its own
    let listener = unsafe { listener.release(&FakeGhost::heap()) }.unwrap_err();
    listener.detach();
    // SAFETY: as above
    let released = unsafe { listener.release(&FakeGhost::heap()) };

    assert!(released.is_ok());
    let freed = crate::support::fake_ghost::HEAP_FREED.lock().unwrap().clone();
    assert_eq!(freed.iter().filter(|block| **block == object).count(), 1);
}
