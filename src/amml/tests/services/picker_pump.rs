use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use alaydriem_stratum::services::PickerPump;
use alaydriem_stratum::types::PickResult;

use crate::support::{DialogSeen, FakeDialog};

#[test]
fn stop_closes_an_open_dialog_and_joins_its_thread() {
    let (dialog, seen) = FakeDialog::blocking();
    let mut pump = PickerPump::new(dialog);
    pump.request();
    assert!(DialogSeen::wait(&seen.shown), "the dialog never opened");

    pump.stop();

    assert!(seen.closed.load(Ordering::SeqCst), "the dialog got no WM_CLOSE");
    assert!(seen.returned.load(Ordering::SeqCst));
    assert!(!pump.has_worker(), "the worker was not joined");
    assert!(!pump.is_busy());
    assert_eq!(pump.deliver(), None, "an answer of a stopped picker is dropped");
}

#[test]
fn stop_right_after_the_request_returns_and_joins() {
    let (dialog, seen) = FakeDialog::blocking();
    let mut pump = PickerPump::new(dialog);
    pump.request();

    pump.stop();

    assert!(!pump.has_worker());
    assert!(seen.picks.load(Ordering::SeqCst) == 0 || seen.closed.load(Ordering::SeqCst));
}

#[test]
fn stop_with_no_picker_open_does_nothing() {
    let (dialog, seen) = FakeDialog::blocking();
    let mut pump = PickerPump::new(dialog);

    pump.stop();

    assert!(!pump.has_worker());
    assert_eq!(seen.picks.load(Ordering::SeqCst), 0);
}

#[test]
fn a_finished_pick_is_delivered_once_and_the_picker_can_open_again() {
    let answer = PickResult { status: 0, paths: vec![r"C:\a.mcstructure".to_string()], dropped: 0, invalid: 0 };
    let (dialog, seen) = FakeDialog::answering(answer.clone());
    let mut pump = PickerPump::new(dialog);
    pump.request();
    let start = Instant::now();
    let delivered = loop {
        if let Some(delivered) = pump.deliver() {
            break delivered;
        }
        assert!(start.elapsed() < Duration::from_secs(10), "no answer arrived");
        std::thread::sleep(Duration::from_millis(5));
    };

    assert_eq!(delivered, answer);
    assert!(!pump.is_busy() && !pump.has_worker());
    assert_eq!(pump.deliver(), None);
    pump.request();
    pump.stop();
    assert!(seen.picks.load(Ordering::SeqCst) >= 1);
}

#[test]
fn a_second_request_while_the_picker_is_open_is_ignored() {
    let (dialog, seen) = FakeDialog::blocking();
    let mut pump = PickerPump::new(dialog);
    pump.request();
    assert!(DialogSeen::wait(&seen.shown));

    pump.request();
    pump.stop();

    assert_eq!(seen.picks.load(Ordering::SeqCst), 1);
}
