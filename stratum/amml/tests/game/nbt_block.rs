use std::ffi::c_void;
use std::sync::atomic::Ordering;

use alaydriem_stratum::game::{BlockRegistry, NbtBlock};
use alaydriem_stratum::types::NbtLookup;
use stratum_core::structs::ghost::GhostPalette;
use stratum_core::structs::structure::{BlockState, BlockStates, StateValue};

use crate::support::FakeGhost;
use crate::support::fake_ghost::{
    KNOWN_NAME, NBT_READ_CALLS, NBT_READ_NULL, NBT_STATE, NBT_STATE_HIGH, TAG_DELETES, TAG_DTOR_CALLS, TAGS_LIVE,
};

// the palette entry bytes of `name` with one int state, as the ghost palette writes them
fn entry_bytes(name: &str) -> Vec<u8> {
    let mut palette = GhostPalette::new();
    palette.entries.push(BlockState::new(
        name.to_string(),
        BlockStates::new(vec![("weirdo_direction".to_string(), StateValue::Int(2))]),
    ));
    let mut out = Vec::new();
    palette.entry_nbt(0, &mut out).unwrap();
    out
}

fn lookup(bytes: &[u8]) -> NbtLookup {
    let calls = FakeGhost::calls();
    let blocks = NbtBlock::new(BlockRegistry::new(&calls), &calls);
    blocks.lookup(bytes)
}

#[test]
fn empty_bytes_are_unknown_with_no_read() {
    assert_eq!(lookup(&[]), NbtLookup::Unknown);
    assert_eq!(FakeGhost::count(&NBT_READ_CALLS), 0);
}

#[test]
fn a_read_tag_is_destroyed_and_freed_exactly_once() {
    let found = lookup(&entry_bytes(KNOWN_NAME));

    assert_eq!(found, NbtLookup::Exact(FakeGhost::known_block() as *const c_void));
    assert_eq!(FakeGhost::count(&NBT_READ_CALLS), 1);
    assert_eq!(FakeGhost::count(&TAG_DTOR_CALLS), 1);
    assert_eq!(FakeGhost::count(&TAG_DELETES), 1);
    assert_eq!(FakeGhost::count(&TAGS_LIVE), 0);
}

#[test]
fn states_the_game_does_not_know_fall_back_to_the_name() {
    NBT_STATE.store(1, Ordering::SeqCst);

    let found = lookup(&entry_bytes(KNOWN_NAME));

    assert_eq!(found, NbtLookup::Default(FakeGhost::name_only_block() as *const c_void));
    assert_eq!(FakeGhost::count(&TAG_DTOR_CALLS), 1);
}

#[test]
fn a_name_the_game_does_not_know_is_unknown() {
    NBT_STATE.store(1, Ordering::SeqCst);

    assert_eq!(lookup(&entry_bytes("minecraft:not_a_block")), NbtLookup::Unknown);
    assert_eq!(FakeGhost::count(&TAG_DTOR_CALLS), 1);
}

#[test]
fn a_read_with_no_tag_destroys_nothing() {
    NBT_READ_NULL.store(true, Ordering::SeqCst);
    NBT_STATE.store(1, Ordering::SeqCst);

    assert_eq!(lookup(&entry_bytes(KNOWN_NAME)), NbtLookup::Default(FakeGhost::name_only_block() as *const c_void));
    assert_eq!(FakeGhost::count(&TAG_DTOR_CALLS), 0);
}

#[test]
fn cut_short_bytes_are_unknown() {
    NBT_STATE.store(1, Ordering::SeqCst);
    let bytes = entry_bytes(KNOWN_NAME);

    assert_eq!(lookup(&bytes[..5]), NbtLookup::Unknown);
    assert_eq!(FakeGhost::count(&TAG_DTOR_CALLS), FakeGhost::count(&NBT_READ_CALLS));
}

#[test]
fn the_state_is_read_as_a_four_byte_int() {
    NBT_STATE_HIGH.store(0xDEAD_BEEF, Ordering::SeqCst);

    assert_eq!(lookup(&entry_bytes(KNOWN_NAME)), NbtLookup::Exact(FakeGhost::known_block() as *const c_void));
}
