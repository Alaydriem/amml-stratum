use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use amm_client::link::ApiTable;
use amm_core::errors::LinkError;
use stratum_core::config::PreviewConfig;
use stratum_core::services::StratumController;
use stratum_core::services::preview::{PreviewService, SourceLocator};
use stratum_core::structs::easy_place::{InventorySlot, SpotWorld, UseCommand, UseProbe};

use super::{HandleState, ManagerSession, SharedGhost, SharedUi};
use crate::traits::UseCore;

// The running core: the manager registration, the controller and what the render thread shares. The game thread
// reaches the state through the lock; the render thread reaches only `shared`, through the frame handler the session
// registered, and `ghost`, through the ghost renderer's take and report.
pub struct StratumHandle {
    state: Mutex<HandleState>,
    shared: Arc<SharedUi>,
    ghost: Arc<SharedGhost>,
    is_poison_reported: AtomicBool,
}

impl StratumHandle {
    // The wait for the preview thread at shutdown.
    pub const PREVIEW_SHUTDOWN_BUDGET: Duration = Duration::from_millis(500);

    // Registers with the manager and starts the controller from the manager's config; an unreadable config
    // gives the defaults. `config_dir` is the mod's config folder and `data_dir` the folder for the library, both
    // absolute paths.
    pub fn connect(table: ApiTable, config_dir: &Path, data_dir: &Path) -> Result<Self, LinkError> {
        // the preview starts before the session, whose frame handler holds it; the manager gives the config only
        // after the registration, so the preview starts with the default bounds and takes the config below
        let preview = PreviewService::new_shared(SourceLocator::locate(data_dir), &PreviewConfig::default());
        let shared = SharedUi::new_shared(Some(preview));
        let session = ManagerSession::connect(table, config_dir, Arc::clone(&shared))?;
        let config = session.config().unwrap_or_default();
        shared.meter().set_enabled(config.debug.telemetry);
        shared.reconfigure_preview(&config.preview);
        let controller = StratumController::new(data_dir.to_path_buf(), config);
        let ghost = SharedGhost::new_shared();
        let state = Mutex::new(HandleState::new(controller, session, Arc::clone(&shared), Arc::clone(&ghost)));
        Ok(Self { state, shared, ghost, is_poison_reported: AtomicBool::new(false) })
    }

    // None when a panic poisoned the lock. The first such call logs it and gives back the library's leases and
    // the cursor, so the player is not left without turning until the game exits; later calls stay quiet.
    pub fn with_state<R>(&self, body: impl FnOnce(&mut HandleState) -> R) -> Option<R> {
        match self.state.lock() {
            Ok(mut state) => Some(body(&mut state)),
            Err(poisoned) => {
                if !self.is_poison_reported.swap(true, Ordering::Relaxed) {
                    curia::error!("stratum lock is poisoned, the library stops until the game restarts", {});
                    let mut state = poisoned.into_inner();
                    if catch_unwind(AssertUnwindSafe(|| state.release_ui())).is_err() {
                        curia::error!("library leases not given back after the panic", {});
                    }
                }
                None
            }
        }
    }

    // The state the frame handler shares with the tick.
    pub fn shared(&self) -> &Arc<SharedUi> {
        &self.shared
    }

    // The ghost frame and reports the render thread shares with the tick.
    pub fn ghost(&self) -> &Arc<SharedGhost> {
        &self.ghost
    }

    // Stops in reverse start order: the controller within its budget, then the session unregisters from the
    // manager, so no frame reaches the preview again, then the preview thread within `PREVIEW_SHUTDOWN_BUDGET`. A
    // poisoned lock still drops the state, so the registration ends either way.
    pub fn shutdown(self) {
        let mut state = match self.state.into_inner() {
            Ok(state) => state,
            Err(poisoned) => poisoned.into_inner(),
        };
        state.shutdown();
        drop(state);
        if let Some(preview) = self.shared.preview()
            && !preview.shutdown(Self::PREVIEW_SHUTDOWN_BUDGET)
        {
            curia::error!("preview thread did not stop before shutdown", {});
        }
    }
}

// the use calls reach the core through its lock; None when a panic poisoned it
impl UseCore for &StratumHandle {
    fn aim(&mut self, probe: &UseProbe) -> Option<Option<[i32; 3]>> {
        self.with_state(|state| state.use_aim(probe))
    }

    fn decide(
        &mut self,
        probe: &UseProbe,
        world: Option<&SpotWorld>,
        inventory: &[InventorySlot<'_>],
        selected: u8,
    ) -> Option<UseCommand> {
        self.with_state(|state| state.use_decide(probe, world, inventory, selected))
    }

    fn released(&mut self) {
        let _ = self.with_state(|state| state.use_released());
    }

    fn not_sent(&mut self, spot: [i32; 3]) {
        let _ = self.with_state(|state| state.use_not_sent(spot));
    }
}
