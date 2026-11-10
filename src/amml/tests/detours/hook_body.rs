use std::cell::Cell;

use alaydriem_stratum::detours::HookBody;
use amml_game::StopLatch;

#[test]
fn a_skip_gives_the_skipped_result_without_the_original() {
    let calls = Cell::new(0);
    let latch = StopLatch::new();

    HookBody::skip_or(|| calls.set(calls.get() + 1), (), &latch, || true);
    let built = HookBody::skip_or(|| 1u8, 0, &latch, || true);

    assert_eq!((calls.get(), built), (0, 0));
}

#[test]
fn otherwise_the_original_runs_once_and_its_result_is_given() {
    let latch = StopLatch::new();

    assert_eq!(HookBody::skip_or(|| 7u8, 0, &latch, || false), 7);
}

#[test]
fn a_raised_latch_runs_the_original_without_asking() {
    let asked = Cell::new(false);
    let latch = StopLatch::new();
    latch.raise();

    let result = HookBody::skip_or(
        || 7u8,
        0,
        &latch,
        || {
            asked.set(true);
            true
        },
    );

    assert_eq!((result, asked.get()), (7, false));
}

#[test]
fn a_panic_in_a_body_raises_the_latch_and_the_original_runs() {
    let latch = StopLatch::new();

    let skipped = HookBody::skip_or(|| 7u8, 0, &latch, || panic!("a test panic"));
    assert_eq!(skipped, 7);
    assert!(latch.raised());

    let latch = StopLatch::new();
    let before = HookBody::before(|| 9u32, &latch, || panic!("a test panic"));
    assert_eq!(before, 9);
    assert!(latch.raised());
}

#[test]
fn before_runs_the_body_then_the_original() {
    let order = std::cell::RefCell::new(Vec::new());
    let latch = StopLatch::new();

    HookBody::before(|| order.borrow_mut().push("original"), &latch, || order.borrow_mut().push("body"));

    assert_eq!(*order.borrow(), vec!["body", "original"]);
}
