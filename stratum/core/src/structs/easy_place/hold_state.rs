// The state of one use key hold.
#[derive(Debug)]
pub(crate) struct HoldState {
    // True once the hold reached a ghost spot.
    pub active: bool,
    // The spot and start time of a swap or pick that waits for the server.
    pub flight: Option<([i32; 3], f64)>,
    // Palette ids already named in a notice during this hold. Oldest dropped past `capacity`.
    notified: Vec<u16>,
    capacity: usize,
}

impl HoldState {
    pub fn new(capacity: usize) -> Self {
        Self { active: false, flight: None, notified: Vec::with_capacity(capacity), capacity }
    }

    // Returns true the first time a palette id is noted in this hold.
    pub fn note(&mut self, palette_id: u16) -> bool {
        if self.notified.contains(&palette_id) {
            return false;
        }
        if self.notified.len() >= self.capacity {
            self.notified.remove(0);
        }
        self.notified.push(palette_id);
        true
    }

    pub fn set_capacity(&mut self, capacity: usize) {
        if self.notified.len() > capacity {
            let surplus = self.notified.len() - capacity;
            self.notified.drain(..surplus);
        }
        self.notified.reserve(capacity - self.notified.len());
        self.capacity = capacity;
    }

    pub fn clear(&mut self) {
        self.active = false;
        self.flight = None;
        self.notified.clear();
    }
}
