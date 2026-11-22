use std::cell::RefCell;
use std::rc::Rc;

use alaydriem_stratum::ghost::{BlockActorKey, BlockActorPool};

type Drops = Rc<RefCell<Vec<u32>>>;

struct FakeActor {
    id: u32,
    drops: Drops,
}

impl FakeActor {
    fn new(id: u32, drops: &Drops) -> Self {
        Self { id, drops: drops.clone() }
    }
}

impl Drop for FakeActor {
    fn drop(&mut self) {
        self.drops.borrow_mut().push(self.id);
    }
}

fn key(entry: u32) -> BlockActorKey {
    BlockActorKey::new(0, entry, None, None)
}

#[test]
fn the_pool_never_holds_more_than_its_capacity() {
    let drops: Drops = Rc::default();
    let mut pool = BlockActorPool::<FakeActor>::new();
    let total = BlockActorPool::<FakeActor>::CAPACITY as u32 + 5;
    for id in 0..total {
        if id % BlockActorPool::<FakeActor>::MAKES_PER_FRAME as u32 == 0 {
            pool.begin_frame();
        }
        assert!(pool.get_or_make(key(id), || Ok(FakeActor::new(id, &drops)), drop).is_some());
    }
    assert_eq!(pool.len(), BlockActorPool::<FakeActor>::CAPACITY);
    assert_eq!(*drops.borrow(), vec![0, 1, 2, 3, 4]);
}

#[test]
fn a_hit_protects_an_actor_from_eviction() {
    let drops: Drops = Rc::default();
    let mut pool = BlockActorPool::<FakeActor>::new();
    let capacity = BlockActorPool::<FakeActor>::CAPACITY as u32;
    for id in 0..capacity {
        pool.begin_frame();
        pool.get_or_make(key(id), || Ok(FakeActor::new(id, &drops)), drop);
    }
    pool.begin_frame();
    assert_eq!(pool.get_or_make(key(0), || Err(true), drop).map(|a| a.id), Some(0));
    pool.get_or_make(key(capacity), || Ok(FakeActor::new(capacity, &drops)), drop);
    assert_eq!(*drops.borrow(), vec![1]);
}

#[test]
fn a_frame_makes_at_most_its_budget() {
    let drops: Drops = Rc::default();
    let mut pool = BlockActorPool::<FakeActor>::new();
    let budget = BlockActorPool::<FakeActor>::MAKES_PER_FRAME as u32;
    let mut made = 0;
    for id in 0..budget + 4 {
        pool.get_or_make(
            key(id),
            || {
                made += 1;
                Ok(FakeActor::new(id, &drops))
            },
            drop,
        );
    }
    assert_eq!(made, budget);
    assert_eq!(pool.len(), budget as usize);
    pool.begin_frame();
    assert!(pool.get_or_make(key(budget), || Ok(FakeActor::new(budget, &drops)), drop).is_some());
}

#[test]
fn a_hit_does_not_spend_budget() {
    let drops: Drops = Rc::default();
    let mut pool = BlockActorPool::<FakeActor>::new();
    pool.get_or_make(key(0), || Ok(FakeActor::new(0, &drops)), drop);
    for _ in 0..BlockActorPool::<FakeActor>::MAKES_PER_FRAME * 3 {
        assert!(pool.get_or_make(key(0), || Err(true), drop).is_some());
    }
    assert!(pool.get_or_make(key(1), || Ok(FakeActor::new(1, &drops)), drop).is_some());
}

#[test]
fn clear_releases_every_actor_once() {
    let drops: Drops = Rc::default();
    let mut pool = BlockActorPool::<FakeActor>::new();
    for id in 0..5 {
        pool.get_or_make(key(id), || Ok(FakeActor::new(id, &drops)), drop);
    }
    pool.clear(drop);
    assert_eq!(pool.len(), 0);
    let mut released = drops.borrow().clone();
    released.sort_unstable();
    assert_eq!(released, vec![0, 1, 2, 3, 4]);
    drop(pool);
    assert_eq!(drops.borrow().len(), 5);
}

#[test]
fn a_failed_make_stores_nothing() {
    let mut pool = BlockActorPool::<FakeActor>::new();
    assert!(pool.get_or_make(key(0), || Err(true), drop).is_none());
    assert!(pool.is_empty());
}

#[test]
fn failing_keys_are_not_retried_and_do_not_block_a_good_key_next_frame() {
    let drops: Drops = Rc::default();
    let mut pool = BlockActorPool::<FakeActor>::new();
    let mut attempts = 0;
    for id in 0..12 {
        pool.get_or_make(
            key(id),
            || {
                attempts += 1;
                Err(true)
            },
            drop,
        );
    }
    assert_eq!(attempts, BlockActorPool::<FakeActor>::MAKES_PER_FRAME);
    pool.begin_frame();
    for id in 0..BlockActorPool::<FakeActor>::MAKES_PER_FRAME as u32 {
        pool.get_or_make(
            key(id),
            || {
                attempts += 1;
                Err(true)
            },
            drop,
        );
    }
    assert_eq!(attempts, BlockActorPool::<FakeActor>::MAKES_PER_FRAME, "remembered failures are not retried");
    assert!(pool.get_or_make(key(100), || Ok(FakeActor::new(100, &drops)), drop).is_some());
}

#[test]
fn failed_memory_replaces_its_oldest_key_when_full() {
    let mut pool = BlockActorPool::<FakeActor>::new();
    let failed = BlockActorPool::<FakeActor>::FAILED_CAPACITY as u32;
    for id in 0..failed + 1 {
        pool.begin_frame();
        pool.get_or_make(key(id), || Err(true), drop);
    }
    pool.begin_frame();
    let mut retried = false;
    pool.get_or_make(
        key(0),
        || {
            retried = true;
            Err(true)
        },
        drop,
    );
    assert!(retried, "the oldest failed key was forgotten");
    let mut retried_newest = false;
    pool.get_or_make(
        key(failed),
        || {
            retried_newest = true;
            Err(true)
        },
        drop,
    );
    assert!(!retried_newest);
}

#[test]
fn clear_forgets_failed_keys() {
    let drops: Drops = Rc::default();
    let mut pool = BlockActorPool::<FakeActor>::new();
    pool.get_or_make(key(1), || Err(true), drop);
    pool.clear(drop);
    assert!(pool.get_or_make(key(1), || Ok(FakeActor::new(1, &drops)), drop).is_some());
}

#[test]
fn the_pool_works_after_clear() {
    let drops: Drops = Rc::default();
    let mut pool = BlockActorPool::<FakeActor>::new();
    let capacity = BlockActorPool::<FakeActor>::CAPACITY as u32;
    for id in 0..capacity {
        pool.begin_frame();
        pool.get_or_make(key(id), || Ok(FakeActor::new(id, &drops)), drop);
    }
    pool.clear(drop);
    for id in 1000..1000 + capacity + 3 {
        pool.begin_frame();
        assert!(pool.get_or_make(key(id), || Ok(FakeActor::new(id, &drops)), drop).is_some());
    }
    assert_eq!(pool.len(), BlockActorPool::<FakeActor>::CAPACITY);
    assert_eq!(pool.get_or_make(key(1003), || Err(true), drop).map(|a| a.id), Some(1003));
    assert!(pool.get_or_make(key(1000), || Err(true), drop).is_none());
}

#[test]
fn lookups_stay_correct_under_heavy_churn() {
    let drops: Drops = Rc::default();
    let mut pool = BlockActorPool::<FakeActor>::new();
    let capacity = BlockActorPool::<FakeActor>::CAPACITY as u32;
    let total = 100_000u32;
    for id in 0..total {
        pool.begin_frame();
        // spread keys over both fields so probe chains collide
        let k = BlockActorKey::new((id % 7) as u16, id / 7, None, None);
        assert!(pool.get_or_make(k, || Ok(FakeActor::new(id, &drops)), drop).is_some());
        if id % 1000 == 999 {
            let live_from = (id + 1).saturating_sub(capacity);
            for probe in live_from..=id {
                let k = BlockActorKey::new((probe % 7) as u16, probe / 7, None, None);
                assert_eq!(pool.get_or_make(k, || Err(true), drop).map(|a| a.id), Some(probe));
            }
            for probe in live_from.saturating_sub(300)..live_from {
                let k = BlockActorKey::new((probe % 7) as u16, probe / 7, None, None);
                pool.begin_frame();
                assert!(pool.get_or_make(k, || Err(true), drop).is_none(), "evicted key {probe} must miss");
            }
        }
    }
    assert_eq!(pool.len(), BlockActorPool::<FakeActor>::CAPACITY);
    assert_eq!(drops.borrow().len() as u32, total - capacity);
}

#[test]
fn more_visible_keys_than_the_capacity_settle_on_a_stable_set_with_no_makes() {
    let drops: Drops = Rc::default();
    let mut pool = BlockActorPool::<FakeActor>::new();
    let capacity = BlockActorPool::<FakeActor>::CAPACITY as u32;
    let budget = BlockActorPool::<FakeActor>::MAKES_PER_FRAME as u32;
    let visible = capacity + 10;
    let warm = capacity.div_ceil(budget) as usize + 1;
    let mut makes_per_frame = Vec::new();
    for _ in 0..warm + 10 {
        pool.begin_frame();
        let mut makes = 0;
        // the same keys in the same order every frame, as the farthest-first draw order gives them
        for id in 0..visible {
            pool.get_or_make(
                key(id),
                || {
                    makes += 1;
                    Ok(FakeActor::new(id, &drops))
                },
                drop,
            );
        }
        makes_per_frame.push(makes);
    }
    assert!(makes_per_frame[warm..].iter().all(|makes| *makes == 0), "makes per frame: {makes_per_frame:?}");
    assert!(drops.borrow().is_empty(), "an actor drawn in a frame is never evicted in that frame");
    assert_eq!(pool.len(), BlockActorPool::<FakeActor>::CAPACITY);
    pool.begin_frame();
    for id in 0..capacity {
        assert_eq!(
            pool.get_or_make(key(id), || Err(true), drop).map(|actor| actor.id),
            Some(id),
            "the same actors stay"
        );
    }
}

#[test]
fn an_evicted_actor_and_a_cleared_pool_go_to_the_release() {
    let drops = Rc::new(RefCell::new(Vec::new()));
    let mut pool = BlockActorPool::<FakeActor>::new();
    let capacity = BlockActorPool::<FakeActor>::CAPACITY as u32;
    for id in 0..capacity {
        pool.begin_frame();
        pool.get_or_make(key(id), || Ok(FakeActor::new(id, &drops)), drop);
    }
    pool.begin_frame();
    let mut released = Vec::new();

    pool.get_or_make(key(capacity), || Ok(FakeActor::new(capacity, &drops)), |old| released.push(old.id));
    pool.clear(|actor| released.push(actor.id));

    assert_eq!(released.len(), 1 + BlockActorPool::<FakeActor>::CAPACITY);
    assert_eq!(released[0], 0);
    assert!(pool.is_empty());
}

#[test]
fn a_make_that_may_work_later_is_tried_again_in_the_next_frame() {
    let drops = Rc::new(RefCell::new(Vec::new()));
    let mut pool = BlockActorPool::<FakeActor>::new();

    assert!(pool.get_or_make(key(1), || Err(false), drop).is_none());
    pool.begin_frame();

    assert!(pool.get_or_make(key(1), || Ok(FakeActor::new(1, &drops)), drop).is_some());
}
