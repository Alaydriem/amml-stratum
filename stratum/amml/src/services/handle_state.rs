use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use stratum_core::services::StratumController;
use stratum_core::structs::easy_place::{InventorySlot, SpotWorld, UseCommand, UseProbe};
use stratum_core::structs::frame::FrameStatus;
use stratum_core::structs::ghost::GhostFrame;
use stratum_core::structs::hud::HudState;

use super::{GhostMeter, ManagerSession, SharedGhost, SharedUi, TickMeter};

// Everything behind the handle's lock, owned by the game thread: the controller and the manager session.
// The controller is declared first, so a plain drop also stops it before the session unregisters.
pub struct HandleState {
    controller: StratumController,
    session: ManagerSession,
    shared: Arc<SharedUi>,
    ghost: Arc<SharedGhost>,
    meter: TickMeter,
    ghost_meter: GhostMeter,
    // True while the HUD keys are live, which is also when the use path may act.
    use_open: bool,
}

impl HandleState {
    const MEGABYTE: u64 = 1024 * 1024;

    pub fn new(
        controller: StratumController,
        session: ManagerSession,
        shared: Arc<SharedUi>,
        ghost: Arc<SharedGhost>,
    ) -> Self {
        shared.publish(controller.snapshot());
        Self {
            controller,
            session,
            shared,
            ghost,
            meter: TickMeter::new(),
            ghost_meter: GhostMeter::new(),
            use_open: false,
        }
    }

    // The manager's events go first, so a close, a world exit or new settings shape this tick. Then the frame's
    // actions, at most one queue's worth, then the Hud lease and its keys, then the world draw lease for the active
    // marker, then the render thread's ghost reports, then the controller's tick, whose snapshot and ghost frame the next frame draws.
    pub fn tick(&mut self, status: &FrameStatus) {
        self.session.sync(status, self.controller.picker_busy());
        if let Some(config) = self.session.take_changed_config() {
            self.shared.reconfigure_preview(&config.preview);
            self.controller.set_ghost_config(config.ghost);
        }
        for _ in 0..SharedUi::ACTION_CAPACITY {
            let Some(action) = self.shared.next_action() else { break };
            self.controller.apply(action);
        }
        self.hud_tick(status);
        let wanted = status.in_world && self.controller.ghost_wanted();
        let allowed = self.session.sync_world_draw(wanted, status.time_seconds);
        if self.session.take_world_draw_revoked() {
            self.controller.ghost_revoked();
        }
        self.controller.set_ghost_allowed(allowed);
        self.ghost.render_meter().set_enabled(self.shared.meter().enabled());
        let Self { controller, ghost, ghost_meter, .. } = self;
        ghost.drain(|report| {
            ghost_meter.observe(report);
            controller.apply_ghost_report(report);
        });
        if self.shared.meter().enabled() {
            self.timed_tick(status);
        } else {
            self.meter.stop();
            self.controller.tick(status);
        }
        self.shared.publish(self.controller.snapshot());
        self.ghost.publish(self.controller.ghost_frame());
    }

    // Holds the Hud lease for an active marker, then applies the HUD keys that arrived while it was held. The card
    // state is published for the frame, or None once the lease is gone.
    fn hud_tick(&mut self, status: &FrameStatus) {
        let has_marker = self.controller.snapshot().markers.iter().any(|marker| marker.active);
        self.session.sync_hud(has_marker, status);
        while let Some(key) = self.session.next_hud_key() {
            self.controller.hud_key(key);
        }
        self.controller.set_hud_held(self.session.hud_keys_held());
        self.shared.publish_hud(self.session.hud_shown().then_some(self.controller.hud_state()));
        self.sync_use_gate();
    }

    // The use path follows the Hud lease: the same rule as the HUD keys. When the gate closes, the hold in
    // progress ends once.
    fn sync_use_gate(&mut self) {
        let open = self.session.hud_shown();
        if self.use_open && !open {
            self.controller.use_released();
        }
        self.use_open = open;
    }

    // The ghost spot under the crosshair; None while the gate is closed.
    pub fn use_aim(&mut self, probe: &UseProbe) -> Option<[i32; 3]> {
        if !self.use_open {
            return None;
        }
        self.controller.use_aim(probe)
    }

    // What to do with this use; vanilla while the gate is closed.
    pub fn use_decide(
        &mut self,
        probe: &UseProbe,
        world: Option<&SpotWorld>,
        inventory: &[InventorySlot<'_>],
        selected: u8,
    ) -> UseCommand {
        if !self.use_open {
            return UseCommand::Vanilla;
        }
        self.controller.use_decide(probe, world, inventory, selected)
    }

    // A block of the world changed at `cell`; `block` is its name.
    pub fn block_seen(&mut self, cell: [i32; 3], block: &[u8], time_seconds: f64) {
        self.controller.block_seen(cell, block, time_seconds);
    }

    // The game did not carry out the placement at `spot`.
    pub fn use_not_sent(&mut self, spot: [i32; 3]) {
        self.controller.use_not_sent(spot);
    }

    // The use key went up.
    pub fn use_released(&mut self) {
        self.controller.use_released();
    }

    // True while the Hud lease is held: the HUD keys are live and the card shows.
    pub fn hud_shown(&self) -> bool {
        self.session.hud_shown()
    }

    // True while a HUD key is down.
    pub fn hud_keys_held(&self) -> bool {
        self.session.hud_keys_held()
    }

    // Where the HUD keys stand.
    pub fn hud_state(&self) -> HudState {
        self.controller.hud_state()
    }

    // A block of the loaded world changed; the ghost section that holds it is rebuilt from the next tick.
    pub fn block_changed(&mut self, position: [i32; 3]) {
        self.ghost_meter.block_changed();
        self.controller.ghost_block_changed(position);
    }

    // The controller's tick with its cost counted. Once a window, writes the telemetry line; this is
    // the only place that allocates for telemetry.
    fn timed_tick(&mut self, status: &FrameStatus) {
        if !self.meter.is_started() {
            // Counts from before telemetry was switched on are not part of the first window.
            let _ = self.controller.take_wakeups();
            let _ = self.shared.meter().take();
            let _ = self.ghost_meter.take();
            let _ = self.controller.take_ghost_tick();
            let _ = self.ghost.render_meter().take();
            self.meter.start(status.time_seconds, self.controller.snapshot().revision, status.feet);
        }
        let started = Instant::now();
        self.controller.tick(status);
        let micros = u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX);
        let snapshot = self.controller.snapshot();
        self.meter.observe(micros, snapshot.revision, status.feet);
        if !self.meter.is_due(status.time_seconds) {
            return;
        }
        let (frames, frame_sum, frame_max) = self.shared.meter().take();
        let wakeups = self.controller.take_wakeups();
        let ghost = self.controller.ghost_telemetry();
        let window = self.ghost_meter.take();
        let ghost_tick = self.controller.take_ghost_tick();
        let render = self.ghost.render_meter().take();
        let easy_place = self.controller.easy_place_telemetry();
        curia::info!("telemetry", {
            ticks: self.meter.ticks(),
            tick_avg_us: self.meter.average_us(),
            tick_max_us: self.meter.max_us(),
            snapshot_rebuilds: self.meter.rebuilds(),
            markers: snapshot.markers.len(),
            player_moved: self.meter.moved(),
            render_frames: render.frames,
            render_frame_avg_us: render.avg_us,
            render_frame_max_us: render.max_us,
            render_frame_p99_us: render.p99_us,
            frames: frames,
            frame_avg_us: frame_sum.checked_div(frames).unwrap_or(0),
            frame_max_us: frame_max,
            store_wakeups: wakeups.store,
            import_wakeups: wakeups.import,
            materials_wakeups: wakeups.materials,
            pending_writes: self.controller.pending_writes(),
            failed_writes: self.controller.failed_writes(),
            failed_write_total: self.controller.store_failures(),
            ghost_sections_live: ghost.sections_live,
            ghost_slot_capacity: ghost.slot_capacity,
            ghost_budget_bytes: ghost.budget_bytes,
            ghost_used_bytes: ghost.used_bytes,
            ghost_used_mb: ghost.used_bytes / Self::MEGABYTE,
            ghost_cut: ghost.cut,
            ghost_turned_bytes: ghost.turned_bytes,
            cache_budget_bytes: ghost.cache_budget_bytes,
            cache_resident_bytes: ghost.cache_resident_bytes,
            ghost_tick_avg_us: ghost_tick.avg_us,
            ghost_tick_max_us: ghost_tick.max_us,
            ghost_frames: window.frames,
            ghost_meshes_drawn: window.meshes_drawn,
            ghost_culled: window.culled,
            ghost_draw_avg_us: window.draw_avg_us,
            ghost_draw_max_us: window.draw_max_us,
            ghost_block_changes: window.block_changes,
            ghost_block_change_overflow: ghost.block_change_overflows,
            ghost_jobs_built: window.jobs_built,
            ghost_cells_built: window.cells_built,
            ghost_skipped_cells: window.skipped_cells,
            ghost_build_frame_max_us: window.build_frame_max_us,
            ghost_unknown_blocks: ghost.unknown_blocks,
            ghost_default_state_blocks: ghost.default_state_blocks,
            ghost_miniature_slots: ghost.miniature_slots,
            ghost_miniature_sections: ghost.miniature_sections,
            ghost_miniature_bytes: ghost.miniature_bytes,
            ghost_miniature_over_limit: ghost.miniature_over_limit,
            ghost_failed_slots: ghost.failed_slots,
            ghost_failed_loads: ghost.failed_loads,
            ghost_reports_dropped: self.ghost.dropped_reports(),
            ghost_report_entries_dropped: self.ghost.dropped_entries(),
            ghost_reports_merged: self.ghost.merged_reports(),
            easy_place_placed: easy_place.placed,
            easy_place_confirmed: easy_place.confirmed,
            easy_place_rejected: easy_place.rejected,
            easy_place_not_sent: easy_place.not_sent,
            easy_place_wrong: easy_place.wrong,
            easy_place_cancelled_missing: easy_place.cancelled_missing,
            easy_place_cancelled_no_neighbour: easy_place.cancelled_no_neighbour,
            easy_place_blocked_vanilla: easy_place.blocked_vanilla,
        });
        self.meter.start(status.time_seconds, snapshot.revision, status.feet);
    }

    pub fn take_picker_request(&mut self) -> bool {
        let pending = self.controller.take_picker_request();
        self.shared.publish(self.controller.snapshot());
        pending
    }

    pub fn files_picked(&mut self, files: Vec<PathBuf>) {
        self.controller.files_picked(files);
        self.shared.publish(self.controller.snapshot());
    }

    // True while the library stays open through the picker and the game's pause screen is not wanted. The
    // controller's busy flag covers the frame between handing out the picker request and the next tick.
    pub fn pause_screen_unwanted(&self) -> bool {
        self.session.pause_screen_unwanted() || self.controller.picker_busy()
    }

    // Imports submitted and not yet answered.
    pub fn pending_imports(&self) -> usize {
        self.controller.pending_imports()
    }

    // Hides the library and the ghost and gives back the leases and the cursor. Called once after a panic
    // poisoned the lock.
    pub fn release_ui(&mut self) {
        self.session.release_ui();
        self.sync_use_gate();
        self.shared.publish_hud(None);
        let mut cleared = GhostFrame::new();
        cleared.clear_all = true;
        self.ghost.publish(&cleared);
    }

    // Stops the controller within its bounded budget. The session unregisters when the state drops.
    pub fn shutdown(&mut self) {
        self.controller.shutdown();
    }
}
