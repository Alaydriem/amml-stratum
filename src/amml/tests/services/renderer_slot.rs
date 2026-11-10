use std::panic::{AssertUnwindSafe, catch_unwind};

use alaydriem_stratum::services::RendererSlot;

use crate::support::fake_ghost::{DESTROYED, RENDERED};
use crate::support::{FakeGhost, GhostRig};

const STONE: &str = "minecraft:stone";

#[test]
fn after_a_render_panic_and_a_new_enable_the_next_frame_draws_and_the_old_meshes_are_freed_once() {
    let mut rig = GhostRig::new();
    let slot = RendererSlot::new();
    slot.install(&FakeGhost::calls());
    let mut frame = GhostRig::ghost_frame(1, 1, &[STONE]);
    frame.jobs = vec![GhostRig::job(0, 1, 0, [0, 64, 0])];
    rig.publish(&frame);
    // serial 1 is the winding probe, serial 2 the job's mesh
    assert!(slot.frame(|renderer| rig.frame_with(renderer, Some(GhostRig::REGION_A))));

    let panicked = catch_unwind(AssertUnwindSafe(|| slot.frame(|_| panic!("a render body that fails"))));
    assert!(panicked.is_err());
    assert!(!slot.frame(|_| ()), "a poisoned slot runs no frame");
    slot.install(&FakeGhost::calls());
    let mut frame = GhostRig::ghost_frame(2, 1, &[STONE]);
    frame.jobs = vec![GhostRig::job(0, 1, 0, [0, 64, 0])];
    frame.draws = vec![GhostRig::draw(0, [0, 64, 0])];
    rig.publish(&frame);

    assert!(slot.frame(|renderer| rig.frame_with(renderer, Some(GhostRig::REGION_A))));

    let destroyed = DESTROYED.lock().unwrap().clone();
    assert_eq!(destroyed.iter().filter(|serial| **serial == 2).count(), 1, "the old mesh is freed once");
    assert!(!RENDERED.lock().unwrap().is_empty(), "the frame after the enable draws");
}

#[test]
fn a_new_enable_without_a_panic_resets_the_renderer_on_its_next_frame() {
    let mut rig = GhostRig::new();
    let slot = RendererSlot::new();
    slot.install(&FakeGhost::calls());
    let mut frame = GhostRig::ghost_frame(1, 1, &[STONE]);
    frame.jobs = vec![GhostRig::job(0, 1, 0, [0, 64, 0])];
    rig.publish(&frame);
    assert!(slot.frame(|renderer| rig.frame_with(renderer, Some(GhostRig::REGION_A))));

    slot.install(&FakeGhost::calls());
    assert!(slot.frame(|_| ()));

    assert_eq!(DESTROYED.lock().unwrap().iter().filter(|serial| **serial == 2).count(), 1);
}
