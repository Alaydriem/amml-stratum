use std::cell::Cell;
use std::ffi::c_void;

use alaydriem_stratum::detours::render_skull::RenderSkull;
use alaydriem_stratum::ghost::SkullSlot;
use alaydriem_stratum::types::SkullArgs;
use amml_game::StopLatch;
use stratum_core::structs::ghost::SkullPose;

const GHOST_NAME: usize = 0x5C0_00E0;
const AIR_NAME: usize = 0xA1_00E0;

// the arguments the game's skull renderer passes for a ghost cell, whose world block is air: the fallback wall face,
// no turn and the air block's name; the breaking amount is a stale stack value
fn game_args() -> SkullArgs {
    SkullArgs {
        this: 0x1000,
        ctx: 0x2000,
        metadata: 0x3000,
        position: 0x4000,
        face: 0,
        degrees: 0.0,
        name: AIR_NAME,
        material: 0x5000,
        texture: 0x6000,
        breaking: 0xDEAD_BEEF_0BAD_F00D,
        animation: -0.0,
        glint: 0,
    }
}

// runs the detour body with `slot` and returns the arguments the original got, and how many times it ran
fn run(slot: &SkullSlot, latch: &StopLatch) -> (Option<SkullArgs>, u32) {
    let (seen, calls) = (Cell::new(None), Cell::new(0));
    let mut args = game_args();
    RenderSkull::run(
        slot,
        &mut args,
        |args| {
            seen.set(Some(*args));
            calls.set(calls.get() + 1);
        },
        latch,
    );
    (seen.get(), calls.get())
}

#[test]
fn a_ghost_skull_draw_gets_the_cell_pose() {
    let slot = SkullSlot::new();
    slot.set(SkullPose::new(1, 45.0), GHOST_NAME as *const c_void);

    let (seen, calls) = run(&slot, &StopLatch::new());

    assert_eq!(calls, 1);
    let expected = SkullArgs { face: 1, degrees: 45.0, name: GHOST_NAME, ..game_args() };
    assert_eq!(seen, Some(expected), "only the face, the turn and the name change");
}

#[test]
fn a_skull_draw_outside_a_ghost_draw_is_untouched() {
    let slot = SkullSlot::new();

    let (seen, calls) = run(&slot, &StopLatch::new());

    assert_eq!((seen, calls), (Some(game_args()), 1));

    slot.set(SkullPose::new(3, 0.0), GHOST_NAME as *const c_void);
    slot.clear();

    assert_eq!(run(&slot, &StopLatch::new()), (Some(game_args()), 1), "a cleared slot changes nothing");
}

#[test]
fn a_raised_latch_leaves_the_arguments_as_the_game_passed_them() {
    let slot = SkullSlot::new();
    slot.set(SkullPose::new(1, 90.0), GHOST_NAME as *const c_void);
    let latch = StopLatch::new();
    latch.raise();

    assert_eq!(run(&slot, &latch), (Some(game_args()), 1));
}

#[test]
fn a_panic_in_the_body_still_calls_the_original_or_skips_safely() {
    let latch = StopLatch::new();
    let (seen, calls) = (Cell::new(None), Cell::new(0));
    let mut args = game_args();

    RenderSkull::guarded(
        &mut args,
        &latch,
        |args| {
            args.face = 4;
            args.name = GHOST_NAME;
            panic!("a swap that fails half way");
        },
        |args| {
            seen.set(Some(*args));
            calls.set(calls.get() + 1);
        },
    );

    assert!(latch.raised(), "the panic stops stratum's render work");
    assert_eq!(calls.get(), 1, "the game still draws its skull");
    assert_eq!(seen.get(), Some(game_args()), "with the arguments the game passed, not a half swap");
}
