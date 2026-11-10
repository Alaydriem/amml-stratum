use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant};

// What a fake dialog did, shared with the test.
#[derive(Default)]
pub struct DialogSeen {
    // the dialog's window exists and its message loop runs
    pub shown: AtomicBool,
    // the window got WM_CLOSE
    pub closed: AtomicBool,
    // pick returned, so the thread is about to end
    pub returned: AtomicBool,
    pub picks: AtomicU32,
}

impl DialogSeen {
    // waits up to 10 s for `flag`
    pub fn wait(flag: &AtomicBool) -> bool {
        let start = Instant::now();
        while !flag.load(Ordering::SeqCst) {
            if start.elapsed() > Duration::from_secs(10) {
                return false;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        true
    }
}
