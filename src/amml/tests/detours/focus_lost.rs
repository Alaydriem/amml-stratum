use std::cell::Cell;

use alaydriem_stratum::detours::focus_lost::FocusLost;
use amml_game::StopLatch;

#[test]
fn while_the_pause_screen_is_not_wanted_the_original_is_not_called() {
    let calls = Cell::new(0);
    let latch = StopLatch::new();

    FocusLost::run(|_| calls.set(calls.get() + 1), 0x90, &latch, || true);

    assert_eq!(calls.get(), 0);
}

#[test]
fn otherwise_the_original_runs_once_with_this() {
    let seen = Cell::new(None);
    let latch = StopLatch::new();

    FocusLost::run(|this| seen.set(Some(this)), 0x90, &latch, || false);

    assert_eq!(seen.get(), Some(0x90));
}

#[test]
fn a_raised_latch_calls_the_original_without_asking() {
    let (calls, asked) = (Cell::new(0), Cell::new(false));
    let latch = StopLatch::new();
    latch.raise();

    FocusLost::run(
        |_| calls.set(calls.get() + 1),
        0x90,
        &latch,
        || {
            asked.set(true);
            true
        },
    );

    assert_eq!(calls.get(), 1);
    assert!(!asked.get());
}

#[test]
fn a_panic_in_the_question_calls_the_original_and_raises_the_latch() {
    let calls = Cell::new(0);
    let latch = StopLatch::new();

    FocusLost::run(|_| calls.set(calls.get() + 1), 0x90, &latch, || panic!("asked"));

    assert_eq!(calls.get(), 1);
    assert!(latch.raised());
}
