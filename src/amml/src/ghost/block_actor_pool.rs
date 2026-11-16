use super::block_actor_index::BlockActorIndex;
use super::block_actor_key::BlockActorKey;

/// A bounded pool of block actors, one per key.
///
/// Holds at most `CAPACITY` actors. A miss at capacity evicts the least recently used actor, which is dropped,
/// unless that actor was used in the current frame: the miss then makes nothing and spends no budget, so more
/// visible keys than the capacity settle on a stable set. A frame makes at most `MAKES_PER_FRAME` actors. A key
/// whose make failed is remembered (the last `FAILED_CAPACITY` of them, oldest replaced) and is not retried until
/// `clear`. Every buffer is allocated in `new`, so no call after `new` allocates.
pub struct BlockActorPool<A> {
    // actors by slot, pushed up to CAPACITY
    actors: Vec<A>,
    // key of each slot, parallel to `actors`
    keys: Vec<BlockActorKey>,
    // last-use stamp of each slot, parallel to `actors`
    stamps: Vec<u64>,
    // key to slot number
    index: BlockActorIndex,
    // keys whose make failed, a ring of at most FAILED_CAPACITY
    failed: Vec<BlockActorKey>,
    // next failed entry to overwrite once `failed` is full
    failed_next: usize,
    // monotonic use counter that orders slots by recency
    clock: u64,
    // the clock when the current frame began; a stamp past it is a use in this frame
    frame_start: u64,
    // makes still allowed this frame
    makes_left: usize,
}

impl<A> BlockActorPool<A> {
    pub const CAPACITY: usize = 256;
    pub const MAKES_PER_FRAME: usize = 8;
    pub const FAILED_CAPACITY: usize = 64;

    pub fn new() -> Self {
        Self {
            actors: Vec::with_capacity(Self::CAPACITY),
            keys: Vec::with_capacity(Self::CAPACITY),
            stamps: Vec::with_capacity(Self::CAPACITY),
            index: BlockActorIndex::new(),
            failed: Vec::with_capacity(Self::FAILED_CAPACITY),
            failed_next: 0,
            clock: 0,
            frame_start: 0,
            makes_left: Self::MAKES_PER_FRAME,
        }
    }

    /// Resets the make budget for a new frame.
    pub fn begin_frame(&mut self) {
        self.makes_left = Self::MAKES_PER_FRAME;
        self.frame_start = self.clock;
    }

    /// The actor for `key` if the pool holds one, marked most recently used. Never makes one.
    pub fn get(&mut self, key: BlockActorKey) -> Option<&A> {
        let slot = self.touch(key)?;
        self.actors.get(slot)
    }

    /// The actor for `key`. A hit marks it most recently used. A miss calls `make` while the frame budget lasts
    /// and keeps the result, evicting the least recently used actor at capacity; when that actor was used in this
    /// frame the miss gives None and spends no budget. A key that failed before is not retried and spends no budget;
    /// a failed make spends budget once.
    pub fn get_or_make(&mut self, key: BlockActorKey, make: impl FnOnce() -> Option<A>) -> Option<&A> {
        if let Some(slot) = self.touch(key) {
            return self.actors.get(slot);
        }
        if self.makes_left == 0 || self.failed.contains(&key) {
            return None;
        }
        let victim = if self.actors.len() < Self::CAPACITY {
            None
        } else {
            let slot = self.least_recently_used()?;
            if *self.stamps.get(slot)? > self.frame_start {
                return None;
            }
            Some(slot)
        };
        self.makes_left -= 1;
        let Some(actor) = make() else {
            self.remember_failure(key);
            return None;
        };
        let slot = if let Some(slot) = victim {
            let old = *self.keys.get(slot)?;
            self.index.remove(&self.keys, &old);
            *self.actors.get_mut(slot)? = actor;
            *self.keys.get_mut(slot)? = key;
            *self.stamps.get_mut(slot)? = self.clock;
            slot
        } else {
            self.actors.push(actor);
            self.keys.push(key);
            self.stamps.push(self.clock);
            self.actors.len() - 1
        };
        self.index.insert(&key, slot);
        self.actors.get(slot)
    }

    /// Drops every actor and forgets every failed key.
    pub fn clear(&mut self) {
        self.index.clear();
        self.actors.clear();
        self.keys.clear();
        self.stamps.clear();
        self.failed.clear();
        self.failed_next = 0;
    }

    pub fn len(&self) -> usize {
        self.actors.len()
    }

    pub fn is_empty(&self) -> bool {
        self.actors.is_empty()
    }

    // the slot of `key`, stamped with a new clock value; None for a miss
    fn touch(&mut self, key: BlockActorKey) -> Option<usize> {
        self.clock += 1;
        let slot = self.index.find(&self.keys, &key)?;
        if let Some(stamp) = self.stamps.get_mut(slot) {
            *stamp = self.clock;
        }
        Some(slot)
    }

    fn remember_failure(&mut self, key: BlockActorKey) {
        if self.failed.len() < Self::FAILED_CAPACITY {
            self.failed.push(key);
        } else if let Some(entry) = self.failed.get_mut(self.failed_next) {
            *entry = key;
            self.failed_next = (self.failed_next + 1) % Self::FAILED_CAPACITY;
        }
    }

    // slot with the smallest stamp
    fn least_recently_used(&self) -> Option<usize> {
        self.stamps.iter().enumerate().min_by_key(|(_, stamp)| **stamp).map(|(slot, _)| slot)
    }
}

impl<A> Default for BlockActorPool<A> {
    fn default() -> Self {
        Self::new()
    }
}
