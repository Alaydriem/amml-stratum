use std::sync::atomic::Ordering;

use alaydriem_stratum::game::{GameBlockTessellator, Tessellator};
use amml_game::BlockPos;

use crate::support::FakeGhost;
use crate::support::fake_ghost::{
    BEGIN_CALLS, BT_CTOR_CALLS, BT_DTOR_CALLS, BT_REGION, END_CALLS, HEAP_LIVE, TESSELLATE_CALLS, TESSELLATE_OK,
};

const REGION: usize = 0x2E610;
const STONE: usize = 0x570E;

#[test]
#[allow(unsafe_code)]
fn a_failed_tessellate_still_pairs_begin_and_end() {
    TESSELLATE_OK.store(false, Ordering::SeqCst);
    let calls = FakeGhost::calls();
    let mut buffer = FakeGhost::tessellator();
    // SAFETY: the fake buffer is laid out as the Tessellator mirror and outlives the wrapper
    let mut tessellator = unsafe { Tessellator::from_raw(buffer.as_mut_ptr(), &calls) }.unwrap();
    // SAFETY: the fake functions take any region
    let mut blocks = unsafe { GameBlockTessellator::new(REGION as *mut _, &calls) }.unwrap();
    let pos = BlockPos { x: 1, y: 70, z: -4 };

    let build = tessellator
        .build_quads(24, "stratum_test", |tessellator| {
            // SAFETY: the fake functions take any block
            unsafe { blocks.tessellate(tessellator, STONE as *const _, pos) }
        })
        .expect("the tessellator was free");

    assert!(!build.filled);
    assert_eq!(FakeGhost::count(&TESSELLATE_CALLS), 1);
    assert_eq!(FakeGhost::count(&BEGIN_CALLS), 1);
    assert_eq!(FakeGhost::count(&END_CALLS), 1);
}

#[test]
#[allow(unsafe_code)]
fn tessellate_outside_a_begin_calls_nothing() {
    let calls = FakeGhost::calls();
    let mut buffer = FakeGhost::tessellator();
    // SAFETY: as above
    let mut tessellator = unsafe { Tessellator::from_raw(buffer.as_mut_ptr(), &calls) }.unwrap();
    // SAFETY: as above
    let mut blocks = unsafe { GameBlockTessellator::new(REGION as *mut _, &calls) }.unwrap();

    // SAFETY: as above
    assert!(!unsafe { blocks.tessellate(&mut tessellator, STONE as *const _, BlockPos { x: 0, y: 0, z: 0 }) });
    assert_eq!(FakeGhost::count(&TESSELLATE_CALLS), 0);
}

#[test]
#[allow(unsafe_code)]
fn new_constructs_on_the_game_heap_and_drop_destroys_and_frees_once() {
    let calls = FakeGhost::calls();
    // SAFETY: as above
    let mut blocks = unsafe { GameBlockTessellator::new(REGION as *mut _, &calls) }.unwrap();
    assert_eq!(FakeGhost::count(&BT_CTOR_CALLS), 1);
    assert_eq!(BT_REGION.load(Ordering::SeqCst), REGION);
    assert_eq!(FakeGhost::count(&HEAP_LIVE), 1);

    // SAFETY: as above
    unsafe { blocks.set_region(0x4E3 as *mut _) };
    assert_eq!(BT_REGION.load(Ordering::SeqCst), 0x4E3);
    drop(blocks);

    assert_eq!(FakeGhost::count(&BT_DTOR_CALLS), 1);
    assert_eq!(FakeGhost::count(&HEAP_LIVE), 0);
}
