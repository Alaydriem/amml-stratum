use crate::ghost::BlockChangeListener;
#[cfg(feature = "gate")]
use crate::services::GatePlacer;
use crate::services::{FileDialog, FrameStatusReader, PickerPump, StratumHandle};
use crate::types::LinkState;

// the per-tick state of one enable, owned by the game thread behind the runtime's lock: the running core, the frame
// status reader with its world key, the import picker, the block listener on the client region and, with the gate,
// the sample placer
pub struct CoreState {
    pub link: LinkState,
    pub handle: Option<StratumHandle>,
    pub frame: FrameStatusReader,
    pub picker: PickerPump<FileDialog>,
    // whether the last tick reached the core, so a failure and a recovery are each logged once
    pub last_tick_ok: bool,
    // made on the game heap at the first world; kept across enables while a source may still hold it
    pub listener: Option<BlockChangeListener>,
    // the listener could not be made, so block changes stay off until the game restarts
    pub listener_failed: bool,
    #[cfg(feature = "gate")]
    pub placer: Option<GatePlacer>,
}

impl CoreState {
    pub fn new() -> Self {
        Self {
            link: LinkState::Off,
            handle: None,
            frame: FrameStatusReader::new(),
            picker: PickerPump::new(FileDialog::new()),
            last_tick_ok: true,
            listener: None,
            listener_failed: false,
            #[cfg(feature = "gate")]
            placer: None,
        }
    }
}

impl Default for CoreState {
    fn default() -> Self {
        Self::new()
    }
}
