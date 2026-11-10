use stratum_core::structs::preview::PreviewSerials;

#[test]
fn a_frame_past_the_newest_request_is_not_an_answer_and_the_view_keeps_waiting() {
    let mut serials = PreviewSerials::new();
    let serial = serials.next();
    serials.sent(serial);
    assert!(!serials.answer(5), "a frame an earlier view asked for counted as an answer");
    assert!(serials.waiting());
    assert!(serials.answer(serial));
    assert!(!serials.waiting());
}

#[test]
fn an_answer_to_an_older_request_keeps_the_view_waiting_for_the_newest() {
    let mut serials = PreviewSerials::new();
    for _ in 0..2 {
        let serial = serials.next();
        serials.sent(serial);
    }
    assert!(serials.answer(1));
    assert!(serials.waiting(), "the answer to request 1 ended the wait for request 2");
    assert!(serials.answer(2));
    assert!(!serials.waiting());
}

#[test]
fn a_late_older_answer_does_not_bring_the_wait_back() {
    let mut serials = PreviewSerials::new();
    serials.sent(serials.next());
    serials.sent(serials.next());
    assert!(serials.answer(2));
    assert!(serials.answer(1));
    assert!(!serials.waiting());
}

#[test]
fn a_view_that_sent_nothing_does_not_wait_and_takes_no_answer() {
    let mut serials = PreviewSerials::new();
    assert!(!serials.waiting());
    assert!(!serials.answer(1));
    assert!(!serials.waiting());
}

#[test]
fn a_request_that_was_not_sent_does_not_move_the_serial() {
    let mut serials = PreviewSerials::new();
    let first = serials.next();
    assert_eq!(serials.next(), first, "asking for the next serial without sending used it up");
    serials.sent(first);
    assert_eq!(serials.next(), first + 1);
}
