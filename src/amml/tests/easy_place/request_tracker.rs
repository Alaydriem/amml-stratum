use alaydriem_stratum::easy_place::RequestTracker;
use amml_api::{ApiError, RequestId, RequestOutcome};
use stratum_core::structs::easy_place::SwapAnswer;

#[test]
fn a_taken_request_waits_for_its_own_done() {
    let mut tracker = RequestTracker::new();

    tracker.sent(Ok(RequestId(4)));
    assert_eq!(tracker.answer(), SwapAnswer::None);
    tracker.done(RequestId(3), RequestOutcome::Done);
    assert_eq!(tracker.answer(), SwapAnswer::None, "another request's done is ignored");
    tracker.done(RequestId(4), RequestOutcome::Done);

    assert_eq!(tracker.answer(), SwapAnswer::Accepted);
    assert_eq!(tracker.awaited(), None);
}

#[test]
fn every_outcome_but_done_is_a_refusal() {
    for outcome in [RequestOutcome::Refused, RequestOutcome::NoAnswer, RequestOutcome::Stale, RequestOutcome::Cancelled]
    {
        let mut tracker = RequestTracker::new();
        tracker.sent(Ok(RequestId(1)));

        tracker.done(RequestId(1), outcome);

        assert_eq!(tracker.answer(), SwapAnswer::Refused, "{outcome:?}");
    }
}

#[test]
fn a_request_the_loader_refused_is_refused_at_once() {
    let mut tracker = RequestTracker::new();

    tracker.sent(Err(ApiError::Refused));

    assert_eq!(tracker.answer(), SwapAnswer::Refused);
    assert_eq!(tracker.awaited(), None);
}
