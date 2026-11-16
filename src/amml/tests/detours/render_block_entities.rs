use std::cell::Cell;

use alaydriem_stratum::detours::render_block_entities::RenderBlockEntities;
use amml_game::StopLatch;

#[test]
fn the_body_runs_after_the_original_on_the_opaque_pass_only() {
    let order = Cell::new(Vec::new());
    let latch = StopLatch::new();

    RenderBlockEntities::run(
        || order.set([order.take(), vec!["opaque original"]].concat()),
        &latch,
        false,
        || order.set([order.take(), vec!["opaque body"]].concat()),
    );
    RenderBlockEntities::run(
        || order.set([order.take(), vec!["alpha original"]].concat()),
        &latch,
        true,
        || order.set([order.take(), vec!["alpha body"]].concat()),
    );

    assert_eq!(order.take(), ["opaque original", "opaque body", "alpha original"]);
}

#[test]
fn a_raised_latch_skips_the_body() {
    let (originals, bodies) = (Cell::new(0), Cell::new(0));
    let latch = StopLatch::new();
    latch.raise();

    RenderBlockEntities::run(|| originals.set(originals.get() + 1), &latch, false, || bodies.set(bodies.get() + 1));

    assert_eq!((originals.get(), bodies.get()), (1, 0));
}

#[test]
fn a_panic_in_the_body_raises_the_latch() {
    let originals = Cell::new(0);
    let latch = StopLatch::new();

    RenderBlockEntities::run(|| originals.set(originals.get() + 1), &latch, false, || panic!("a body that fails"));

    assert!(latch.raised());
    assert_eq!(originals.get(), 1, "the game's own pass ran");
}
