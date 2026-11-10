use std::cell::Cell;
use std::sync::atomic::Ordering;

use alaydriem_stratum::detours::render_level::RenderLevel;
use amml_game::StopLatch;

use crate::ghost::ghost_renderer::assert_paired;
use crate::support::FakeGhost;
use crate::support::GhostRig;
use crate::support::fake_ghost::{BEGIN_CALLS, LATCH, TESSELLATE_CALLS, TESSELLATE_RAISE_AT};

#[test]
fn the_body_runs_after_the_original_while_the_latch_is_down() {
    let order = Cell::new(Vec::new());
    let latch = StopLatch::new();

    RenderLevel::run(
        || order.set([order.take(), vec!["original"]].concat()),
        &latch,
        || order.set([order.take(), vec!["body"]].concat()),
    );

    assert_eq!(order.take(), ["original", "body"]);
}

#[test]
fn after_a_disable_raised_the_latch_during_a_frame_the_next_frame_runs_no_body() {
    let (originals, bodies) = (Cell::new(0), Cell::new(0));
    let latch = StopLatch::new();

    RenderLevel::run(
        || originals.set(originals.get() + 1),
        &latch,
        || {
            bodies.set(bodies.get() + 1);
            latch.raise();
        },
    );
    RenderLevel::run(|| originals.set(originals.get() + 1), &latch, || bodies.set(bodies.get() + 1));

    assert_eq!((originals.get(), bodies.get()), (2, 1));
}

#[test]
fn a_panic_in_the_body_raises_the_latch() {
    let latch = StopLatch::new();

    RenderLevel::run(|| (), &latch, || panic!("a render body that fails"));

    assert!(latch.raised());
}

#[test]
fn a_disable_that_raises_the_stop_latch_during_a_ghost_build_ends_every_begin_and_stops_the_next_frame() {
    // the detour body is the ghost frame; the fake game raises the latch the detour reads in the middle of the job's
    // tessellation, as a disable on another thread does (call 1 is the winding probe at the join, call 2 the job)
    TESSELLATE_RAISE_AT.store(2, Ordering::SeqCst);
    let mut rig = GhostRig::new();
    let mut frame = GhostRig::ghost_frame(1, 1, &["minecraft:stone"]);
    frame.jobs = vec![GhostRig::job(0, 1, 0, [0, 64, 0])];
    rig.publish(&frame);
    let bodies = Cell::new(0);

    RenderLevel::run(
        || (),
        &LATCH,
        || {
            bodies.set(bodies.get() + 1);
            rig.frame(Some(GhostRig::REGION_A));
        },
    );
    assert!(LATCH.raised(), "the disable came during the frame");
    assert_paired(&rig);
    let begins = FakeGhost::count(&BEGIN_CALLS);
    let tessellations = FakeGhost::count(&TESSELLATE_CALLS);
    let originals = Cell::new(0);
    RenderLevel::run(
        || originals.set(originals.get() + 1),
        &LATCH,
        || {
            bodies.set(bodies.get() + 1);
            rig.frame(Some(GhostRig::REGION_A));
        },
    );

    assert_eq!(bodies.get(), 1, "no body runs after the disable");
    assert_eq!(originals.get(), 1, "the game's own render still runs");
    assert_eq!((FakeGhost::count(&BEGIN_CALLS), FakeGhost::count(&TESSELLATE_CALLS)), (begins, tessellations));
}
