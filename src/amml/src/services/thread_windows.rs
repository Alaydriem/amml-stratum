use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{EnumThreadWindows, PostMessageW, WM_CLOSE};
use windows::core::BOOL;

// The windows of one thread: the picker closes its dialog through them
pub struct ThreadWindows;

impl ThreadWindows {
    // the OS id of the calling thread
    #[allow(unsafe_code)]
    pub fn current() -> u32 {
        // SAFETY: GetCurrentThreadId has no preconditions
        unsafe { GetCurrentThreadId() }
    }

    // posts WM_CLOSE to every top-level window of the thread `thread_id`; a modal dialog answers it as a cancel
    #[allow(unsafe_code)]
    pub fn post_close(thread_id: u32) {
        // SAFETY: the callback has the WNDENUMPROC signature and reads no state through the LPARAM; a thread that
        // has ended has no windows, so the call then does nothing
        let _ = unsafe { EnumThreadWindows(thread_id, Some(Self::close_window), LPARAM(0)) };
    }

    #[allow(unsafe_code)]
    unsafe extern "system" fn close_window(window: HWND, _: LPARAM) -> BOOL {
        // SAFETY: `window` is a window EnumThreadWindows hands to the callback; posting to a window that closed
        // in the meantime fails and is ignored
        let _ = unsafe { PostMessageW(Some(window), WM_CLOSE, WPARAM(0), LPARAM(0)) };
        BOOL(1)
    }
}
