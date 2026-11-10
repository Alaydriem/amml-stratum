use alaydriem_stratum::services::BlockChangeQueue;
use alaydriem_stratum::types::QueuedChange;

fn change(x: i32, name: Option<&str>) -> QueuedChange {
    QueuedChange { position: [x, 64, 0], seen: name.map(|name| (name.as_bytes().to_vec(), 1.5)) }
}

#[test]
fn changes_queued_while_the_core_is_held_reach_the_next_tick_in_order_with_their_names() {
    let queue = BlockChangeQueue::new();
    queue.push(change(1, Some("minecraft:stone")));
    queue.push(change(2, None));

    let mut drained = Vec::new();
    queue.drain(|change| drained.push(change));

    assert_eq!(drained, [change(1, Some("minecraft:stone")), change(2, None)]);
    let mut again = 0;
    queue.drain(|_| again += 1);
    assert_eq!(again, 0, "a drained change is not handed out twice");
}

#[test]
fn a_change_past_the_capacity_is_dropped_and_counted() {
    let queue = BlockChangeQueue::new();
    for x in 0..BlockChangeQueue::CAPACITY as i32 {
        assert!(queue.push(change(x, None)));
    }

    assert!(!queue.push(change(-1, None)));

    assert_eq!(queue.take_dropped(), 1);
    assert_eq!(queue.take_dropped(), 0);
    let mut kept = 0;
    queue.drain(|_| kept += 1);
    assert_eq!(kept, BlockChangeQueue::CAPACITY);
}

#[test]
fn clear_drops_every_waiting_change() {
    let queue = BlockChangeQueue::new();
    queue.push(change(1, None));

    queue.clear();

    let mut left = 0;
    queue.drain(|_| left += 1);
    assert_eq!(left, 0);
}
