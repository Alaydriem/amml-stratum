use super::PendingPlace;

// Placements that wait for the server. Holds at most `capacity` entries, oldest first. When it is
// full the oldest entry is evicted. Entries are reused, so a push allocates nothing once warm.
#[derive(Debug)]
pub(crate) struct PendingSet {
    entries: Vec<PendingPlace>,
    len: usize,
}

impl PendingSet {
    pub fn new(capacity: usize) -> Self {
        let mut entries = Vec::with_capacity(capacity);
        entries.resize_with(capacity, PendingPlace::new);
        Self { entries, len: 0 }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn contains(&self, spot: [i32; 3]) -> bool {
        self.entries[..self.len].iter().any(|entry| entry.spot == spot)
    }

    // Adds an entry. Returns true when the oldest entry was evicted to make room.
    pub fn push(&mut self, spot: [i32; 3], expected: &str, slab: &str, first_half: bool, deadline: f64) -> bool {
        if self.entries.is_empty() {
            return false;
        }
        let mut evicted = false;
        if self.len == self.entries.len() {
            self.entries[..self.len].rotate_left(1);
            self.len -= 1;
            evicted = true;
        }
        let entry = &mut self.entries[self.len];
        entry.spot = spot;
        entry.expected.clear();
        entry.expected.push_str(expected);
        entry.seen.clear();
        entry.slab.clear();
        entry.slab.push_str(slab);
        entry.first_half = first_half;
        entry.deadline = deadline;
        self.len += 1;
        evicted
    }

    // Records the block name seen at a pending spot. Once the expected block was seen the entry is
    // settled and a later change does not undo it. The first half of a double slab settles on the
    // single slab and can then only move on to the double slab.
    pub fn record_seen(&mut self, spot: [i32; 3], block: &[u8]) {
        let Ok(name) = std::str::from_utf8(block) else {
            return;
        };
        for entry in &mut self.entries[..self.len] {
            if entry.spot != spot {
                continue;
            }
            let expected_seen = !entry.seen.is_empty() && entry.seen == entry.expected;
            let half_seen = entry.first_half && !entry.slab.is_empty() && entry.seen == entry.slab;
            if !expected_seen && (!half_seen || name == entry.expected) {
                entry.seen.clear();
                entry.seen.push_str(name);
            }
        }
    }

    // Removes the entry at `spot`. Returns true when there was one.
    pub fn remove(&mut self, spot: [i32; 3]) -> bool {
        let Some(index) = self.entries[..self.len].iter().position(|entry| entry.spot == spot) else {
            return false;
        };
        self.entries[index..self.len].rotate_left(1);
        self.len -= 1;
        true
    }

    // Moves the oldest entry whose deadline has passed into `out` and removes it.
    pub fn pop_due(&mut self, time_seconds: f64, out: &mut PendingPlace) -> bool {
        let Some(index) = self.entries[..self.len].iter().position(|entry| entry.deadline <= time_seconds) else {
            return false;
        };
        std::mem::swap(out, &mut self.entries[index]);
        self.entries[index..self.len].rotate_left(1);
        self.len -= 1;
        true
    }

    // Changes the capacity. Returns how many of the oldest entries were evicted.
    pub fn set_capacity(&mut self, capacity: usize) -> usize {
        let mut evicted = 0;
        if capacity < self.len {
            evicted = self.len - capacity;
            self.entries[..self.len].rotate_left(evicted);
            self.len = capacity;
        }
        if capacity < self.entries.len() {
            self.entries.truncate(capacity);
        } else {
            self.entries.resize_with(capacity, PendingPlace::new);
        }
        evicted
    }
}
