// Spots already tried in this hold. Holds at most `capacity` spots. When it is full the oldest
// spot is dropped.
#[derive(Debug)]
pub(crate) struct TriedSet {
    spots: Vec<[i32; 3]>,
    capacity: usize,
}

impl TriedSet {
    pub fn new(capacity: usize) -> Self {
        Self { spots: Vec::with_capacity(capacity), capacity }
    }

    pub fn len(&self) -> usize {
        self.spots.len()
    }

    pub fn contains(&self, spot: [i32; 3]) -> bool {
        self.spots.contains(&spot)
    }

    pub fn push(&mut self, spot: [i32; 3]) {
        if self.spots.contains(&spot) {
            return;
        }
        if self.spots.len() >= self.capacity {
            self.spots.remove(0);
        }
        self.spots.push(spot);
    }

    pub fn remove(&mut self, spot: [i32; 3]) {
        self.spots.retain(|known| *known != spot);
    }

    pub fn set_capacity(&mut self, capacity: usize) {
        if self.spots.len() > capacity {
            let surplus = self.spots.len() - capacity;
            self.spots.drain(..surplus);
        }
        self.spots.reserve(capacity - self.spots.len());
        self.capacity = capacity;
    }

    pub fn clear(&mut self) {
        self.spots.clear();
    }
}
