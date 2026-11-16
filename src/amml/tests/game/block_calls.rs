use std::sync::atomic::Ordering;

use alaydriem_stratum::game::BlockCalls;

use crate::support::FakeGhost;
use crate::support::fake_ghost::{EXTRA_LAYER_MASK, EXTRA_LAYERS, RENDER_LAYER, RENDER_LAYER_CALLS};

#[test]
#[allow(unsafe_code)]
fn a_null_block_or_type_makes_no_layer_call_and_reads_no_layers() {
    let calls = BlockCalls::new(&FakeGhost::calls());

    // SAFETY: null is the documented no-call input
    let (main, extra) = unsafe { (calls.render_layer(std::ptr::null()), calls.extra_render_layers(std::ptr::null())) };

    assert_eq!((main, extra), (0, 0));
    assert_eq!(RENDER_LAYER_CALLS.load(Ordering::SeqCst), 0);
}

#[test]
#[allow(unsafe_code)]
fn a_block_gives_its_main_layer_and_its_type_gives_the_extra_mask() {
    EXTRA_LAYER_MASK.store(EXTRA_LAYERS, Ordering::SeqCst);
    let calls = BlockCalls::new(&FakeGhost::calls());
    let block = FakeGhost::known_block() as *const std::ffi::c_void;

    // SAFETY: a fake block laid out as the game's, with a fake block type and vtable
    let (main, extra) = unsafe { (calls.render_layer(block), calls.extra_render_layers(calls.block_type(block))) };

    assert_eq!((main, extra), (RENDER_LAYER, EXTRA_LAYERS));
}
