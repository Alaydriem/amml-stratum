use alaydriem_stratum::easy_place::StackRequest;
use stratum_core::structs::easy_place::SwapAnswer;

use crate::support::FakeUse;
use crate::support::fake_packet::{SENT_IDS, STREAM_DTOR_CALLS};
use crate::support::fake_use::STACK_DTOR_CALLS;

const PICK_ID: i32 = StackRequest::FIRST_REQUEST_ID;
const INVENTORY_TYPE: i8 = -1;
const INTERACT: u32 = 33;
const CONTAINER_CLOSE: u32 = 47;
const ITEM_STACK_REQUEST: u32 = 147;

fn count(line: &str) -> usize {
    FakeUse::log().iter().filter(|seen| *seen == line).count()
}

#[allow(unsafe_code)]
fn picked(rig: &FakeUse, request: &mut StackRequest, now: f64) {
    let block = FakeUse::palette_block(FakeUse::PLANKS_PALETTE_ID);
    // SAFETY: the rig's fake client, player and block
    unsafe { request.pick(now, rig.client, rig.player, block, 0) };
    assert_eq!(request.answer(), SwapAnswer::None);
    assert_eq!(rig.slot(0), (FakeUse::planks(), 64), "the pick is predicted");
}

#[allow(unsafe_code)]
fn swapped(rig: &FakeUse, request: &mut StackRequest, now: f64) {
    // SAFETY: the rig's fake client and player
    unsafe { request.swap(now, rig.client, rig.player, 9, 0) };
    assert_eq!(request.answer(), SwapAnswer::None);
    assert_eq!(rig.slot(0), (FakeUse::stone(), 64), "the swap is predicted");
}

#[test]
fn a_pick_opens_the_inventory_then_sends_and_predicts_with_a_copy_kept_for_the_undo() {
    let rig = FakeUse::new();
    let mut request = StackRequest::new(&rig.packets, &rig.players, &rig.calls);

    picked(&rig, &mut request, 1.0);

    assert_eq!(*SENT_IDS.lock().unwrap(), vec![INTERACT, ITEM_STACK_REQUEST]);
    assert_eq!(FakeUse::log(), vec!["copy_stack".to_string(), "set_item 0".to_string()]);
    // the predicted stack is destroyed once it is set; the undo copy is kept
    assert_eq!(FakeUse::count(&STACK_DTOR_CALLS), 1);
    assert_eq!(request.awaited(), PICK_ID);
}

#[test]
fn an_unanswered_pick_is_undone_once_after_the_timeout_and_its_undo_stack_destroyed_once() {
    let rig = FakeUse::new();
    let mut request = StackRequest::new(&rig.packets, &rig.players, &rig.calls);
    picked(&rig, &mut request, 1.0);

    request.expire(1.0 + StackRequest::REQUEST_TIMEOUT_SECONDS - 0.01);
    assert_eq!(count("set_item 0"), 1, "no undo before the timeout");

    request.expire(1.0 + StackRequest::REQUEST_TIMEOUT_SECONDS);
    request.expire(5.0);

    assert_eq!(count("set_item 0"), 2, "one undo");
    assert_eq!(rig.slot(0), (FakeUse::dirt(), 10));
    assert_eq!(FakeUse::count(&STACK_DTOR_CALLS), 2, "the predicted stack and the undo stack, each once");
    assert_eq!(request.answer(), SwapAnswer::Refused);
    assert_eq!(request.awaited(), 0, "the next probe does not wait for an answer");
}

#[test]
fn a_refused_pick_is_undone_once_and_its_undo_stack_destroyed_once() {
    let rig = FakeUse::new();
    let mut request = StackRequest::new(&rig.packets, &rig.players, &rig.calls);
    picked(&rig, &mut request, 1.0);

    request.on_response(FakeUse::response(PICK_ID, 3, None));
    request.on_response(FakeUse::response(PICK_ID, 3, None));
    request.expire(5.0);

    assert_eq!(count("set_item 0"), 2, "one undo");
    assert_eq!(rig.slot(0), (FakeUse::dirt(), 10));
    assert_eq!(FakeUse::count(&STACK_DTOR_CALLS), 2);
    assert_eq!(request.answer(), SwapAnswer::Refused);
}

#[test]
fn an_unanswered_or_refused_swap_is_swapped_back_once() {
    for refused in [false, true] {
        FakeUse::clear_log();
        let rig = FakeUse::new();
        let mut request = StackRequest::new(&rig.packets, &rig.players, &rig.calls);
        swapped(&rig, &mut request, 1.0);
        let id = request.awaited();

        if refused {
            request.on_response(FakeUse::response(id, 7, None));
        }
        request.expire(5.0);
        request.expire(6.0);

        assert_eq!(count("swap_slots 0 9"), 2, "the prediction and one undo (refused: {refused})");
        assert_eq!(rig.slot(0), (FakeUse::dirt(), 10));
        assert_eq!(rig.slot(9), (FakeUse::stone(), 64));
        assert_eq!(request.answer(), SwapAnswer::Refused);
    }
}

#[test]
fn an_undo_leaves_a_slot_that_no_longer_holds_the_predicted_item() {
    let rig = FakeUse::new();
    let mut request = StackRequest::new(&rig.packets, &rig.players, &rig.calls);
    picked(&rig, &mut request, 1.0);
    // the player moved another stack into the slot before the timeout
    rig.fill(0, FakeUse::stone(), 5);

    request.expire(5.0);

    assert_eq!(count("set_item 0"), 1, "no undo");
    assert_eq!(rig.slot(0), (FakeUse::stone(), 5));
    assert_eq!(FakeUse::count(&STACK_DTOR_CALLS), 2, "the undo stack is still destroyed once");
}

#[test]
fn an_accepted_pick_takes_the_server_net_id_only_while_the_slot_holds_the_pick() {
    let rig = FakeUse::new();
    let mut request = StackRequest::new(&rig.packets, &rig.players, &rig.calls);
    picked(&rig, &mut request, 1.0);

    request.on_response(FakeUse::response(PICK_ID, 0, Some((0, 555))));

    assert_eq!(request.answer(), SwapAnswer::Accepted);
    assert_eq!(rig.net_id(0), Some(555));
    assert_eq!(count("set_item 0"), 1, "no undo");

    let mut request = StackRequest::new(&rig.packets, &rig.players, &rig.calls);
    picked(&rig, &mut request, 2.0);
    rig.fill(0, FakeUse::stone(), 5);

    request.on_response(FakeUse::response(PICK_ID, 0, Some((0, 666))));

    assert_eq!(rig.net_id(0), Some(100), "a slot that no longer holds the pick keeps its net id");
}

#[test]
fn only_the_inventory_open_easy_place_asked_for_is_hidden_once() {
    let rig = FakeUse::new();
    let mut request = StackRequest::new(&rig.packets, &rig.players, &rig.calls);
    assert!(!request.on_container_open(0.5, FakeUse::open_packet(3, INVENTORY_TYPE)), "nothing was asked for");
    picked(&rig, &mut request, 1.0);

    // a chest the user opens at the same moment
    assert!(!request.on_container_open(1.1, FakeUse::open_packet(4, 0)));
    assert!(request.on_container_open(1.2, FakeUse::open_packet(5, INVENTORY_TYPE)));
    assert!(!request.on_container_open(1.3, FakeUse::open_packet(5, INVENTORY_TYPE)), "hidden once");

    assert_eq!(SENT_IDS.lock().unwrap().last(), Some(&CONTAINER_CLOSE), "the close for the hidden open");
    assert!(!request.on_container_close(1.4, FakeUse::close_packet(4)), "the user's own container closes");
    assert!(request.on_container_close(1.5, FakeUse::close_packet(5)));
    assert!(!request.on_container_close(1.6, FakeUse::close_packet(5)), "hidden once");
}

#[test]
fn an_open_after_the_answer_window_goes_to_the_game() {
    let rig = FakeUse::new();
    let mut request = StackRequest::new(&rig.packets, &rig.players, &rig.calls);
    picked(&rig, &mut request, 1.0);

    assert!(!request.on_container_open(3.5, FakeUse::open_packet(5, INVENTORY_TYPE)));
}

#[test]
fn a_reset_undoes_the_request_in_flight_once() {
    let rig = FakeUse::new();
    let mut request = StackRequest::new(&rig.packets, &rig.players, &rig.calls);
    swapped(&rig, &mut request, 1.0);

    request.reset();
    request.reset();

    assert_eq!(count("swap_slots 0 9"), 2);
    assert_eq!(request.answer(), SwapAnswer::None);
    assert_eq!(FakeUse::count(&STREAM_DTOR_CALLS), 1);
}
