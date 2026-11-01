use std::ffi::c_void;
use std::sync::atomic::Ordering;

use alaydriem_stratum::ghost::BlockResolver;
use stratum_core::structs::ghost::GhostPalette;
use stratum_core::structs::structure::{BlockState, BlockStates, StateValue};

use crate::support::FakeGhost;
use crate::support::fake_ghost::{KNOWN_NAME, NBT_READ_CALLS, NBT_STATE, RENDER_LAYER, TAG_DTOR_CALLS, TAGS_LIVE};

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

fn joined() -> BlockResolver {
    let mut blocks = BlockResolver::new(&FakeGhost::calls());
    assert!(blocks.join(), "air and stone are known");
    blocks
}

fn unknown() -> *const c_void {
    FakeGhost::block("minecraft:unknown") as *const c_void
}

#[test]
fn empty_cut_short_and_unknown_entries_each_give_the_unknown_block_and_count_once() {
    NBT_STATE.store(1, Ordering::SeqCst);
    let mut blocks = joined();
    let full = entry_bytes(KNOWN_NAME);
    let unknown_name = entry_bytes("minecraft:not_a_block");
    let entries: [&[u8]; 3] = [&[], &full[..5], &unknown_name];

    for entry in entries {
        blocks.resolve([entry].into_iter(), 0);

        assert_eq!(blocks.block(0), unknown());
        assert_eq!(blocks.take_unknown(), 1);
    }
    assert_eq!(blocks.take_default_state(), 0);
}

#[test]
fn every_tag_the_read_made_is_destroyed_once() {
    NBT_STATE.store(1, Ordering::SeqCst);
    let mut blocks = joined();
    let full = entry_bytes(KNOWN_NAME);
    let unknown_name = entry_bytes("minecraft:not_a_block");

    blocks.resolve([&[][..], &full[..5], &unknown_name, &full].into_iter(), 0);

    assert_eq!(FakeGhost::count(&NBT_READ_CALLS), 3, "an empty entry is not read");
    assert_eq!(FakeGhost::count(&TAG_DTOR_CALLS), 3);
    assert_eq!(FakeGhost::count(&TAGS_LIVE), 0);
    assert_eq!(blocks.take_unknown(), 3);
    assert_eq!(blocks.take_default_state(), 1);
}

#[test]
fn a_resolved_entry_keeps_its_graphics_and_render_layer() {
    let mut blocks = joined();
    let entry = entry_bytes(KNOWN_NAME);

    blocks.resolve([&entry[..]].into_iter(), 1);

    assert_eq!(blocks.block(0), FakeGhost::known_block() as *const c_void);
    assert_eq!(blocks.graphics(0), FakeGhost::graphics() as *const c_void);
    assert_eq!(blocks.layer(0), i32::from(RENDER_LAYER));
    assert!(blocks.block(1).is_null(), "an id past the palette has no block");
    assert!(blocks.block(u16::MAX).is_null(), "an empty cell has no block");
}

#[test]
fn leave_forgets_every_block_of_the_world() {
    let mut blocks = joined();
    let entry = entry_bytes(KNOWN_NAME);
    blocks.resolve([&entry[..]].into_iter(), 0);

    blocks.leave();

    assert!(blocks.block(0).is_null());
    assert!(blocks.air().is_null());
}
