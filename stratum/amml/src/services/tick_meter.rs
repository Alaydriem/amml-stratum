// Telemetry for the game thread's tick, one window of `WINDOW_SECONDS` of game time. The caller
// measures the controller call and reads the clock; this holds only the numbers, so a tick that
// is not logged allocates nothing.
#[derive(Debug)]
pub struct TickMeter {
    next_at: f64,
    ticks: u64,
    sum_us: u64,
    max_us: u64,
    rebuilds: u64,
    last_revision: u64,
    moved: bool,
    last_feet: [i32; 3],
}

impl TickMeter {
    pub const WINDOW_SECONDS: f64 = 2.0;

    pub fn new() -> Self {
        Self {
            next_at: f64::NEG_INFINITY,
            ticks: 0,
            sum_us: 0,
            max_us: 0,
            rebuilds: 0,
            last_revision: 0,
            moved: false,
            last_feet: [0; 3],
        }
    }

    // Starts a window at `now` with the current revision and position as its baseline.
    pub fn start(&mut self, now: f64, revision: u64, feet: [i32; 3]) {
        self.next_at = now + Self::WINDOW_SECONDS;
        self.ticks = 0;
        self.sum_us = 0;
        self.max_us = 0;
        self.rebuilds = 0;
        self.moved = false;
        self.last_revision = revision;
        self.last_feet = feet;
    }

    // True before the first window and after a pause in telemetry.
    pub fn is_started(&self) -> bool {
        self.next_at.is_finite()
    }

    // Forgets the window, so the next `observe` caller starts a new one.
    pub fn stop(&mut self) {
        self.next_at = f64::NEG_INFINITY;
    }

    // One tick that took `micros`, ending with snapshot `revision` and the player at `feet`.
    pub fn observe(&mut self, micros: u64, revision: u64, feet: [i32; 3]) {
        self.ticks += 1;
        self.sum_us += micros;
        self.max_us = self.max_us.max(micros);
        if revision != self.last_revision {
            self.rebuilds += 1;
            self.last_revision = revision;
        }
        if feet != self.last_feet {
            self.moved = true;
            self.last_feet = feet;
        }
    }

    pub fn is_due(&self, now: f64) -> bool {
        now >= self.next_at
    }

    pub fn ticks(&self) -> u64 {
        self.ticks
    }

    pub fn average_us(&self) -> u64 {
        self.sum_us.checked_div(self.ticks).unwrap_or(0)
    }

    pub fn max_us(&self) -> u64 {
        self.max_us
    }

    pub fn rebuilds(&self) -> u64 {
        self.rebuilds
    }

    pub fn moved(&self) -> bool {
        self.moved
    }
}

impl Default for TickMeter {
    fn default() -> Self {
        Self::new()
    }
}
