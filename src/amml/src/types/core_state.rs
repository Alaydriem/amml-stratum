use std::sync::Arc;

use amm_client::link::ManagerLink;
use stratum_core::structs::build::BuildConditions;

use crate::build::BuildExecutor;
use crate::services::{FileDialog, FrameStatusReader, PickerPump, SharedUi, StratumHandle};
use crate::traits::{UseCore, UseCoreHolder};

// the per-tick state of one enable, owned by the game thread behind the runtime's lock: the manager link until it
// connects, then the running core, the frame status reader with its world key, the import picker and the build
pub struct CoreState {
    // the link started at enable, until the manager's tables are there and the core starts on it
    pub link: Option<ManagerLink>,
    // the state the link's frame handler was made with, for the core
    pub shared: Arc<SharedUi>,
    pub handle: Option<StratumHandle>,
    pub frame: FrameStatusReader,
    pub picker: PickerPump<FileDialog>,
    // whether the last tick reached the core, so a failure and a recovery are each logged once
    pub last_tick_ok: bool,
    // reads the build conditions and answers the core's requests; stopped while the build is off
    pub build: BuildExecutor,
    // the conditions of the last tick
    pub conditions: BuildConditions,
    // the link was found gone and the line was logged
    pub gone_logged: bool,
}

impl CoreState {
    pub fn new(link: ManagerLink, shared: Arc<SharedUi>, build: BuildExecutor) -> Self {
        Self {
            link: Some(link),
            shared,
            handle: None,
            frame: FrameStatusReader::new(),
            picker: PickerPump::new(FileDialog::new()),
            last_tick_ok: true,
            build,
            conditions: BuildConditions::default(),
            gone_logged: false,
        }
    }
}

// the running core answers the use calls; None while the link has not connected
impl UseCoreHolder for CoreState {
    fn use_core(&mut self) -> Option<&mut dyn UseCore> {
        self.handle.as_mut().map(|handle| handle as &mut dyn UseCore)
    }
}
