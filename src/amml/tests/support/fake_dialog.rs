use std::sync::Arc;
use std::sync::atomic::Ordering;

use alaydriem_stratum::traits::DialogRunner;
use alaydriem_stratum::types::PickResult;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, GetMessageW, MSG, WINDOW_EX_STYLE, WM_CLOSE, WS_OVERLAPPED,
};
use windows::core::w;

use super::DialogSeen;

// A dialog runner that stands in for the file dialog: with `answer` None it makes a hidden window on the picker's
// thread and blocks in a message loop until that window gets WM_CLOSE, then answers as a cancel; with `answer` set
// it returns that answer at once
pub struct FakeDialog {
    pub seen: Arc<DialogSeen>,
    pub answer: Option<PickResult>,
}

impl FakeDialog {
    pub fn blocking() -> (Self, Arc<DialogSeen>) {
        let seen = Arc::new(DialogSeen::default());
        (Self { seen: Arc::clone(&seen), answer: None }, seen)
    }

    pub fn answering(answer: PickResult) -> (Self, Arc<DialogSeen>) {
        let seen = Arc::new(DialogSeen::default());
        (Self { seen: Arc::clone(&seen), answer: Some(answer) }, seen)
    }

    fn block_until_closed(&self) {
        // SAFETY: the STATIC class is built in; the window has no parent and belongs to this thread
        let window = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE(0),
                w!("STATIC"),
                w!("fake dialog"),
                WS_OVERLAPPED,
                0,
                0,
                1,
                1,
                None,
                None,
                None,
                None,
            )
        }
        .expect("the fake dialog window");
        self.seen.shown.store(true, Ordering::SeqCst);
        let mut message = MSG::default();
        // SAFETY: `message` is a live MSG; the loop reads this thread's queue only
        while unsafe { GetMessageW(&mut message, None, 0, 0) }.0 > 0 {
            if message.message == WM_CLOSE {
                self.seen.closed.store(true, Ordering::SeqCst);
                break;
            }
        }
        // SAFETY: the window belongs to this thread and is destroyed once
        let _ = unsafe { DestroyWindow(window) };
    }
}

impl DialogRunner for FakeDialog {
    fn pick(&self) -> PickResult {
        self.seen.picks.fetch_add(1, Ordering::SeqCst);
        let result = match &self.answer {
            Some(answer) => answer.clone(),
            None => {
                self.block_until_closed();
                PickResult::cancelled()
            }
        };
        self.seen.returned.store(true, Ordering::SeqCst);
        result
    }
}
