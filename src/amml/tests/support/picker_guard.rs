use stratum_core::structs::action::LibraryAction;

use super::CoreSession;

// Drives the picker the way the tick and the library do.
pub struct PickerGuard;

impl PickerGuard {
    // The library asked for the picker on an in-world tick at `time` and the tick opened it.
    pub fn start(session: &CoreSession, time: f64) {
        session.shared().send(&mut vec![LibraryAction::RequestFilePicker(None)]);
        session.tick(&CoreSession::in_world_at(time));
        assert!(Self::take_request(session));
    }

    // The picker was cancelled: the controller is idle again.
    pub fn cancel(session: &CoreSession) {
        session.handle().with_state(|state| state.files_picked(Vec::new()));
    }

    pub fn unwanted(session: &CoreSession) -> bool {
        session.handle().with_state(|state| state.pause_screen_unwanted()).unwrap_or(false)
    }

    // True once after the library asked for the picker.
    pub fn take_request(session: &CoreSession) -> bool {
        session.handle().with_state(|state| state.take_picker_request()).unwrap_or(false)
    }
}
