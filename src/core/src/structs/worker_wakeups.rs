// How often each background thread went round its loop in one telemetry window. A thread that
// waits on its queue shows about 20 a second; a thread that spins shows thousands.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WorkerWakeups {
    pub store: u64,
    pub import: u64,
    pub materials: u64,
}
