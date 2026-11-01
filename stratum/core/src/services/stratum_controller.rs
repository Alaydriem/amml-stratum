use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use super::materials_job::MaterialsJob;
use super::{
    EasyPlaceService, GhostService, HudInput, ImportWorker, InboxScanner, ItemRules, LayoutSolver, LibraryService,
    LibraryStore, MarkerService, MarkerStore, MaterialCounter, MaterialsWorker, PlaceNeeds, StoreWriter,
    StructureCache,
};
use crate::config::{GhostConfig, ImportConfig, StratumConfig};
use crate::errors::{ImportBusy, LibraryError, MarkerError, StoreError, StructureError};
use crate::structs::action::{HudAction, LibraryAction, MaterialScope};
use crate::structs::easy_place::{
    EasyPlaceNotice, EasyPlaceTelemetry, InventorySlot, PlaceNeed, SpotWorld, UseCommand, UseProbe,
};
use crate::structs::frame::FrameStatus;
use crate::structs::ghost::{GhostFrame, GhostReport, GhostTelemetry, GhostTickMeter, GhostTickWindow};
use crate::structs::hud::{HudKey, HudState, HudTarget};
use crate::structs::import::{ImportFailure, ImportRequest, ImportResult};
use crate::structs::layout::{BigBuildLayout, LayoutIssue};
use crate::structs::library::{Library, NewStructure, Project, ProjectId, ProjectKind, StructureEntry, StructureId};
use crate::structs::marker::{Marker, MarkerSet, MarkerTarget, Rotation, WorldKey};
use crate::structs::material::{ItemId, MaterialList, StructureLoad, StructureLoaded};
use crate::structs::snapshot::{
    HudRow, ImportNotice, ImportPick, LibrarySnapshot, MarkerRow, ProjectRow, StructureRow,
};
use crate::structs::structure::Structure;
use crate::structs::worker_wakeups::WorkerWakeups;

// The one entry point the FFI layer calls on the game thread. It owns the library, the markers
// of the loaded world, the import worker and the inbox scanner, builds the snapshot the UI reads
// and applies the UI's actions. Nothing here waits for another thread: imports, structure reads
// for material lists and file writes are queued, and their results are picked up on later ticks.
// Bound: at most `NOTICE_LIMIT` notices are kept, each for `NOTICE_SECONDS`; the oldest is dropped
// first. At most `2 * import.queue + 1` inbox paths are in flight, because the worker refuses
// requests past its queue. Inbox sources in flight plus those waiting for the library file to reach
// the disk are at most `library.max_structures`; the inbox is not scanned while that many wait.
pub struct StratumController {
    // The folder of the stored structure files.
    structures_dir: PathBuf,
    config: StratumConfig,
    library: LibraryService,
    library_store: LibraryStore,
    library_path: PathBuf,
    // False when the library file could not be read, so a save cannot overwrite it.
    library_writable: bool,
    markers: Option<MarkerService>,
    marker_store: MarkerStore,
    // False when the marker file of the loaded world could not be read.
    markers_writable: bool,
    cache: StructureCache,
    rules: ItemRules,
    worker: Option<ImportWorker>,
    store_writer: Option<StoreWriter>,
    materials_worker: Option<MaterialsWorker>,
    // The material list being built, if the structures were not all cached.
    materials_job: Option<MaterialsJob>,
    materials_failed: Option<MaterialScope>,
    inbox: InboxScanner,
    // Inbox paths submitted and not yet answered.
    inbox_in_flight: HashSet<PathBuf>,
    // Picked paths submitted and not yet answered, with the serial and the requested destination of
    // their pick. Bound: the import queue, because a refused submit is not recorded.
    picked_in_flight: HashMap<PathBuf, (u64, Option<ProjectId>)>,
    // Inbox sources whose structure the library accepted. Each is removed once the library file
    // that names its structure is on disk.
    awaiting_save: Vec<PathBuf>,
    pending_imports: usize,
    scan_buffer: Vec<PathBuf>,
    result_buffer: Vec<ImportResult>,
    loaded_buffer: Vec<StructureLoaded>,
    // Time the notice was raised, and the notice.
    notices: VecDeque<(f64, ImportNotice)>,
    materials: Option<(MaterialScope, MaterialList)>,
    picker_pending: bool,
    // The project the pending or open picker request imports into.
    picker_destination: Option<ProjectId>,
    // True from handing out a picker request until its answer arrives.
    picker_open: bool,
    // The most recent picker import, and the serial of the last pick started.
    latest_pick: Option<ImportPick>,
    pick_serial: u64,
    status: FrameStatus,
    next_inbox_poll: f64,
    inbox_poll_seconds: f64,
    // The earliest time of the next save request.
    next_save_at: f64,
    // The earliest time to queue failed writes again, and the wait after the next failure.
    retry_at: f64,
    backoff_seconds: f64,
    seen_failures: u64,
    ticked: bool,
    snapshot: Arc<LibrarySnapshot>,
    // The target and the time of the last marker placed, so a repeated click places nothing.
    last_place: Option<(MarkerTarget, f64)>,
    changed: bool,
    // The library part of the snapshot (projects, structures, layouts) needs a rebuild.
    library_changed: bool,
    ghost: GhostService,
    ghost_frame: GhostFrame,
    // The time of `GhostService::update` per tick, read out by `take_ghost_tick`.
    ghost_tick: GhostTickMeter,
    ghost_allowed: bool,
    // The structures the active ghost build needs, parsed. Bound: `ghost.needed()`; entries that
    // are no longer needed are dropped after every ghost update.
    ghost_loaded: HashMap<StructureId, Arc<Structure>>,
    // Needed structures asked of the materials worker and not yet answered.
    ghost_in_flight: HashSet<StructureId>,
    // Needed structures that could not be read. Retried after a library change.
    ghost_failed: HashSet<StructureId>,
    // Where the HUD keys stand; kept for the session, never saved.
    hud: HudState,
    easy_place: EasyPlaceService,
    // The placement needs of the ghost build with the given revision. Rebuilt by the tick when the
    // build changes. Bound: one entry, sized by the palette of the build.
    place_needs: Option<(u64, PlaceNeeds)>,
    // The latest easy place notice, the time it was taken and, for a missing block, the item it needs.
    // Bound: one entry.
    easy_place_notice: Option<(f64, EasyPlaceNotice, Option<ItemId>)>,
}

impl StratumController {
    pub const NOTICE_LIMIT: usize = 16;
    pub const NOTICE_SECONDS: f64 = 10.0;
    pub const SAVE_INTERVAL_SECONDS: f64 = 1.0;
    pub const MAX_BACKOFF_SECONDS: f64 = 30.0;
    const MIN_POLL_SECONDS: f64 = 0.1;
    // A second place for the same target within this time is the same click: the second action of one frame, a
    // frame before the menu shows the first marker, or the second click of a double click (egui reads 0.3 s).
    pub const PLACE_REPEAT_SECONDS: f64 = 0.5;
    // The most `shutdown` waits for the writes and the store thread together.
    pub const SHUTDOWN_BUDGET: Duration = Duration::from_secs(5);

    pub fn new(data_dir: PathBuf, config: StratumConfig) -> StratumController {
        Self::with_materials_worker(data_dir, config, MaterialsWorker::new())
    }

    pub fn with_materials_worker(
        data_dir: PathBuf,
        config: StratumConfig,
        materials_worker: MaterialsWorker,
    ) -> StratumController {
        let library_store = LibraryStore::new(data_dir.clone());
        let library_path = library_store.path();
        let existed = library_path.exists();
        let (library, writable, cleanup) = match library_store.load() {
            // A file that was set aside as corrupt is gone: its structures are not orphans.
            Ok(library) => (library, true, !existed || library_path.exists()),
            Err(error) => {
                curia::error!("library file could not be read", { error: error.to_string() });
                (Library::new(), false, false)
            }
        };
        if cleanup {
            Self::remove_orphans(&data_dir, &library);
        }
        if let Err(error) = std::fs::create_dir_all(data_dir.join("import")) {
            curia::error!("inbox folder could not be created", { error: error.to_string() });
        }

        let import = ImportConfig::from_settings(&config.import);
        let inbox_poll_seconds = f64::from(import.inbox_poll_seconds).max(Self::MIN_POLL_SECONDS);
        let mut controller = Self {
            library: LibraryService::new(library, config.library.clone()),
            library_store,
            library_path,
            library_writable: writable,
            markers: None,
            marker_store: MarkerStore::new(data_dir.join("markers")),
            markers_writable: true,
            cache: StructureCache::new(config.cache.budget_bytes()),
            rules: ItemRules::new(),
            store_writer: Some(StoreWriter::new()),
            worker: Some(ImportWorker::new(data_dir.clone(), import)),
            materials_worker: Some(materials_worker),
            materials_job: None,
            materials_failed: None,
            inbox: InboxScanner::new(data_dir.join("import")),
            inbox_in_flight: HashSet::new(),
            picked_in_flight: HashMap::new(),
            awaiting_save: Vec::new(),
            pending_imports: 0,
            scan_buffer: Vec::new(),
            result_buffer: Vec::new(),
            loaded_buffer: Vec::new(),
            notices: VecDeque::new(),
            materials: None,
            picker_pending: false,
            picker_open: false,
            picker_destination: None,
            latest_pick: None,
            pick_serial: 0,
            status: FrameStatus::default(),
            next_inbox_poll: f64::NEG_INFINITY,
            inbox_poll_seconds,
            next_save_at: f64::NEG_INFINITY,
            retry_at: f64::NEG_INFINITY,
            backoff_seconds: Self::SAVE_INTERVAL_SECONDS,
            seen_failures: 0,
            ticked: false,
            snapshot: Arc::new(LibrarySnapshot::empty()),
            last_place: None,
            changed: false,
            library_changed: false,
            ghost: GhostService::new(config.ghost.clone()),
            ghost_frame: GhostFrame::new(),
            ghost_tick: GhostTickMeter::new(),
            ghost_allowed: true,
            ghost_loaded: HashMap::new(),
            ghost_in_flight: HashSet::new(),
            ghost_failed: HashSet::new(),
            hud: HudState::new(),
            easy_place: EasyPlaceService::new(config.easy_place.clone()),
            place_needs: None,
            easy_place_notice: None,
            structures_dir: data_dir.join("structures"),
            config,
        };
        let mut first = controller.library_part();
        controller.set_small_parts(&mut first, Vec::new(), None);
        controller.snapshot = Arc::new(first);
        controller
    }

    pub fn tick(&mut self, status: &FrameStatus) {
        let moved = status.feet != self.status.feet;
        let was_in_world = self.status.in_world;
        let had_player = self.status.has_player;
        self.status.clone_from(status);
        // A world whose key the shim has not decided yet counts as no world: no marker set is
        // loaded or saved under an empty key, and placing a marker stays off.
        if self.status.world.is_empty() {
            self.status.in_world = false;
        }
        if self.status.in_world != was_in_world || self.status.has_player != had_player {
            self.changed = true;
        }
        if !self.ticked {
            self.ticked = true;
            if !self.library_writable {
                self.notify(ImportNotice::LibraryUnavailable);
            }
        }
        if self.status.menu_open {
            self.easy_place.end_hold();
        }
        self.easy_place.expire(self.status.time_seconds);
        self.sync_world();
        self.check_movement(moved);
        self.poll_inbox();
        self.drain_results();
        self.drain_loaded();
        self.expire_notices();
        self.expire_easy_place_notice();
        self.save_if_due();
        self.release_saved_sources();
        self.update_ghost();
        self.refresh();
    }

    pub fn apply(&mut self, action: LibraryAction) {
        match action {
            LibraryAction::ImportFiles(files) => self.submit_picked(files, None),
            LibraryAction::RequestFilePicker(destination) => {
                if !self.picker_open {
                    self.picker_pending = true;
                    self.picker_destination = destination;
                    self.changed = true;
                }
            }
            LibraryAction::CreateProject(name) => {
                let result = self.library.create_project(&name).map(|_| ());
                self.library_outcome(result);
            }
            LibraryAction::RenameProject(id, name) => {
                let result = self.library.rename_project(id, &name);
                self.library_outcome(result);
            }
            LibraryAction::DeleteProject(id) => {
                let result = self.library.delete_project(id);
                self.drop_materials();
                self.library_outcome(result);
            }
            LibraryAction::SetProjectKind(id, kind) => {
                let result = self.library.set_project_kind(id, kind);
                self.library_outcome(result);
            }
            LibraryAction::RenameStructure(id, name) => {
                let result = self.library.rename_structure(id, &name);
                self.library_outcome(result);
            }
            LibraryAction::MoveStructure(id, project) => {
                let result = self.library.move_structure(id, project);
                self.drop_materials();
                self.library_outcome(result);
            }
            LibraryAction::DeleteStructure(id) => self.delete_structure(id),
            LibraryAction::SetGridCell(id, cell) => {
                let result = self.library.set_grid_cell(id, cell);
                self.library_outcome(result);
            }
            LibraryAction::PlaceMarker { name, target } => self.place_marker(&name, target),
            LibraryAction::ActivateMarker(id) => self.change_markers(|markers| markers.activate(id)),
            LibraryAction::DeactivateMarker => self.change_markers(|markers| {
                markers.deactivate();
                Ok(())
            }),
            LibraryAction::RenameMarker(id, name) => self.change_markers(|markers| markers.rename(id, &name)),
            LibraryAction::AssignMarker(id, target) => self.change_markers(|markers| markers.assign(id, target)),
            LibraryAction::DeleteMarker(id) => self.change_markers(|markers| markers.delete(id)),
            LibraryAction::LoadMaterials(scope) => self.load_materials(scope),
        }
        self.refresh();
    }

    // One HUD key press: moves the mode or the carousel, or applies the action the key makes to the active
    // marker. The mode and the carousel move even with no active marker.
    pub fn hud_key(&mut self, key: HudKey) {
        let has_structures = self.hud_has_structures();
        if let Some(action) = HudInput::press(&mut self.hud, key, has_structures) {
            self.apply_hud(action);
        }
    }

    // Where the HUD keys stand.
    pub fn hud_state(&self) -> HudState {
        self.hud
    }

    // Applies a key action to the active marker. Without an active marker nothing happens. The
    // change is saved by the next marker save.
    pub fn apply_hud(&mut self, action: HudAction) {
        let Some(active) = self.markers.as_ref().and_then(MarkerService::active) else {
            return;
        };
        // the layouts of the last snapshot are current: they change only with the library
        let target = self.hud_target(active, &self.snapshot.layouts);
        let (layers, structures) = (target.layers, target.structures);
        let Some(markers) = self.markers.as_mut() else {
            return;
        };
        match markers.apply_hud(action, layers, structures) {
            Ok(()) => self.changed = true,
            Err(error) => curia::error!("marker action could not be applied", { error: error.to_string() }),
        }
    }

    // True when the active marker targets a normal project of two or more structures, so the Structure
    // control has something to step through.
    pub fn hud_has_structures(&self) -> bool {
        let Some(active) = self.markers.as_ref().and_then(MarkerService::active) else {
            return false;
        };
        self.hud_target(active, &self.snapshot.layouts).structures >= 2
    }

    // Step one of a use call: the ghost spot under the crosshair, if easy place is on for the active
    // marker and the ghost is drawn and settled. The shim reads the world around the spot and calls
    // `use_decide` next.
    pub fn use_aim(&mut self, probe: &UseProbe) -> Option<[i32; 3]> {
        let enabled =
            self.markers.as_ref().and_then(MarkerService::active).is_some_and(|marker| marker.view.easy_place);
        let build = if enabled {
            match (self.ghost.aim_view(), &self.place_needs) {
                (Some((build, view)), Some((revision, needs))) if *revision == build.revision => {
                    Some((build, view, needs))
                }
                _ => None,
            }
        } else {
            None
        };
        self.easy_place.aim(probe, build)
    }

    // Step two of a use call: what the shim does with this use.
    pub fn use_decide(
        &mut self,
        probe: &UseProbe,
        world: Option<&SpotWorld>,
        inventory: &[InventorySlot<'_>],
        selected: u8,
    ) -> UseCommand {
        let command = self.easy_place.decide(probe, world, inventory, selected);
        if let Some(notice) = self.easy_place.take_notice() {
            let item = match (&notice, &self.place_needs) {
                (EasyPlaceNotice::Missing { palette_id }, Some((_, needs))) => match needs.need(*palette_id) {
                    PlaceNeed::Place { item, .. } => Some(item.clone()),
                    PlaceNeed::NotTarget => None,
                },
                _ => None,
            };
            self.easy_place_notice = Some((self.status.time_seconds, notice, item));
            self.changed = true;
        }
        command
    }

    // A block of the world changed at `cell`; `block` is its name.
    pub fn block_seen(&mut self, cell: [i32; 3], block: &[u8], time_seconds: f64) {
        self.easy_place.seen(cell, block, time_seconds);
    }

    // The shim did not carry out the placement at `spot`.
    pub fn use_not_sent(&mut self, spot: [i32; 3]) {
        self.easy_place.not_sent(spot);
    }

    // The use key went up, or a gate closed.
    pub fn use_released(&mut self) {
        self.easy_place.end_hold();
    }

    pub fn easy_place_telemetry(&self) -> EasyPlaceTelemetry {
        self.easy_place.telemetry()
    }

    // The ghost frame of the last tick. It stays valid until the next tick.
    pub fn ghost_frame(&self) -> &GhostFrame {
        &self.ghost_frame
    }

    // True while a HUD key is down: a moved ghost waits for the release before it is built at its new place.
    pub fn set_hud_held(&mut self, held: bool) {
        self.ghost.set_keys_held(held);
    }

    // False when the world draw lease is not held. The ghost then draws nothing, and the marker
    // stays active.
    pub fn set_ghost_allowed(&mut self, allowed: bool) {
        self.ghost_allowed = allowed;
    }

    // True while a marker is active and shown, so the ghost has something to draw.
    pub fn ghost_wanted(&self) -> bool {
        self.markers.as_ref().and_then(MarkerService::active).is_some_and(|marker| marker.view.shown)
    }

    // The shim's answers for the jobs of the last ghost frame.
    pub fn apply_ghost_report(&mut self, report: &GhostReport) {
        self.ghost.apply_report(report);
    }

    // A block of the loaded world changed at `position`.
    pub fn ghost_block_changed(&mut self, position: [i32; 3]) {
        self.ghost.block_changed(position);
    }

    // The ghost settings changed while the game runs. Draw distance, cells per
    // frame and the mini ghost apply from the next tick; the mesh budget waits for a restart.
    pub fn set_ghost_config(&mut self, config: GhostConfig) {
        self.ghost.reconfigure(config);
        self.config.ghost = self.ghost.config().clone();
    }

    // The world draw lease was taken back: the ghost frees every mesh slot on the next tick.
    pub fn ghost_revoked(&mut self) {
        self.ghost.drop_slots();
    }

    pub fn ghost_telemetry(&self) -> GhostTelemetry {
        GhostTelemetry {
            sections_live: self.ghost.live_slots(),
            slot_capacity: self.ghost.slot_capacity(),
            budget_bytes: self.ghost.budget_bytes(),
            used_bytes: self.ghost.used_bytes(),
            turned_bytes: self.ghost.turned_bytes(),
            cache_budget_bytes: self.cache.budget_bytes(),
            cache_resident_bytes: self.cache.resident_bytes(),
            cut: self.ghost.cut(),
            failed_slots: self.ghost.failed_slots(),
            failed_loads: u32::try_from(self.ghost_failed.len()).unwrap_or(u32::MAX),
            block_change_overflows: self.ghost.block_change_overflows(),
            unknown_blocks: self.ghost.unknown_blocks(),
            default_state_blocks: self.ghost.default_state_blocks(),
            miniature_slots: self.ghost.miniature_slots(),
            miniature_sections: self.ghost.miniature_sections(),
            miniature_bytes: self.ghost.miniature_bytes(),
            miniature_over_limit: self.ghost.miniature_over_limit(),
        }
    }

    // The core ghost work per tick since the last call, and a fresh window.
    pub fn take_ghost_tick(&mut self) -> GhostTickWindow {
        self.ghost_tick.take()
    }

    pub fn snapshot(&self) -> Arc<LibrarySnapshot> {
        Arc::clone(&self.snapshot)
    }

    // True once after a `RequestFilePicker` action. The caller opens the picker and answers with `files_picked`.
    // Until that answer the picker counts as open: the snapshot keeps `picker_pending` and a new request is ignored.
    pub fn take_picker_request(&mut self) -> bool {
        let pending = std::mem::take(&mut self.picker_pending);
        if pending {
            self.picker_open = true;
            self.changed = true;
            self.refresh();
        }
        pending
    }

    // The picker's answer, empty when it was cancelled. Picked files are copied into the library. The sources
    // stay where they are.
    pub fn files_picked(&mut self, files: Vec<PathBuf>) {
        self.picker_open = false;
        self.changed = true;
        if !files.is_empty() {
            self.pick_serial += 1;
            let destination = self.picker_destination;
            self.latest_pick = Some(ImportPick::new(self.pick_serial, destination));
            self.submit_picked(files, Some((self.pick_serial, destination)));
        }
        self.refresh();
    }

    // True while a picker request is pending or open.
    pub fn picker_busy(&self) -> bool {
        self.picker_pending || self.picker_open
    }

    // Imports submitted and not yet answered.
    pub fn pending_imports(&self) -> usize {
        self.pending_imports
    }

    // Bytes the structure cache holds.
    pub fn cache_resident_bytes(&self) -> u64 {
        self.cache.resident_bytes()
    }

    // Write failures since the start.
    pub fn store_failures(&self) -> u64 {
        self.store_writer.as_ref().map_or(0, StoreWriter::failures)
    }

    // Log lines written for write failures: one per failure streak.
    pub fn store_failure_logs(&self) -> u64 {
        self.store_writer.as_ref().map_or(0, StoreWriter::failure_logs)
    }

    // Loop passes of each background thread since the last call; the counts restart at zero.
    pub fn take_wakeups(&self) -> WorkerWakeups {
        WorkerWakeups {
            store: self.store_writer.as_ref().map_or(0, StoreWriter::take_wakeups),
            import: self.worker.as_ref().map_or(0, ImportWorker::take_wakeups),
            materials: self.materials_worker.as_ref().map_or(0, MaterialsWorker::take_wakeups),
        }
    }

    // Paths waiting to be written.
    pub fn pending_writes(&self) -> usize {
        self.store_writer.as_ref().map_or(0, StoreWriter::pending)
    }

    // Writes that failed and wait for a retry.
    pub fn failed_writes(&self) -> usize {
        self.store_writer.as_ref().map_or(0, StoreWriter::failed)
    }

    // Waits until queued saves reached the disk or the timeout passes. Blocks: for shutdown paths and
    // tests, never for the frame.
    pub fn flush_saves(&self, timeout: Duration) -> bool {
        self.store_writer.as_ref().is_none_or(|writer| writer.flush(timeout))
    }

    // Queues dirty state, stops the workers in reverse start order, and waits a bounded time for
    // the writes. Calling it again does nothing.
    pub fn shutdown(&mut self) {
        self.shutdown_within(Self::SHUTDOWN_BUDGET);
    }

    // Like `shutdown`, with the time for the writes and the store thread given.
    pub fn shutdown_within(&mut self, budget: Duration) {
        let started = Instant::now();
        self.ghost.shutdown();
        self.save_library();
        self.save_markers();
        self.materials_job = None;
        self.materials_worker = None;
        self.worker = None;
        if let Some(mut writer) = self.store_writer.take() {
            writer.retry_failed();
            if !writer.flush(budget) {
                curia::error!("saves did not finish before shutdown", {});
            }
            self.release_saved_sources_with(&writer);
            writer.close(budget.saturating_sub(started.elapsed()));
        }
        self.pending_imports = 0;
        self.inbox_in_flight.clear();
        self.picked_in_flight.clear();
        // Sources whose structure did not reach the library file stay in the inbox for the next start.
        self.awaiting_save.clear();
    }

    fn sync_world(&mut self) {
        let same = match (&self.markers, self.status.in_world) {
            (None, false) => true,
            (Some(markers), true) => {
                let world = &markers.set().world;
                world.world == self.status.world && world.dimension == self.status.dimension
            }
            _ => false,
        };
        if same {
            return;
        }
        self.save_markers();
        self.markers = None;
        self.markers_writable = true;
        if self.status.in_world {
            let key = WorldKey { world: self.status.world.clone(), dimension: self.status.dimension };
            let set = match self.load_markers(&key) {
                Ok(set) => set,
                Err(error) => {
                    curia::error!("marker file could not be read", { error: error.to_string() });
                    self.markers_writable = false;
                    self.notify(ImportNotice::MarkersUnavailable);
                    MarkerSet::new(key)
                }
            };
            self.markers = Some(MarkerService::new(set, self.config.markers.clone()));
        }
        self.changed = true;
    }

    // Bytes still waiting in the store writer are newer than the file.
    fn load_markers(&self, key: &WorldKey) -> Result<MarkerSet, StoreError> {
        let waiting = self.store_writer.as_ref().and_then(|writer| writer.latest(&self.marker_store.path_of(key)));
        match waiting {
            Some(bytes) => self.marker_store.decode(key, &bytes),
            None => self.marker_store.load(key),
        }
    }

    // Nothing is imported while the library file cannot be written, because nothing could be saved.
    fn poll_inbox(&mut self) {
        if self.status.time_seconds < self.next_inbox_poll {
            return;
        }
        self.next_inbox_poll = self.status.time_seconds + self.inbox_poll_seconds;
        let waiting = self.awaiting_save.len() + self.inbox_in_flight.len();
        if !self.library_writable || waiting >= self.config.library.max_structures {
            return;
        }
        let mut found = std::mem::take(&mut self.scan_buffer);
        found.clear();
        self.inbox.scan(&mut found);
        for path in found.drain(..) {
            match self.submit(path.clone()) {
                Ok(()) => {
                    self.inbox_in_flight.insert(path);
                }
                Err(ImportBusy) => self.inbox.release(&path),
            }
        }
        self.scan_buffer = found;
    }

    fn submit(&mut self, source: PathBuf) -> Result<(), ImportBusy> {
        let worker = self.worker.as_ref().ok_or(ImportBusy)?;
        worker.submit(ImportRequest { source })?;
        self.pending_imports += 1;
        Ok(())
    }

    fn submit_picked(&mut self, files: Vec<PathBuf>, pick: Option<(u64, Option<ProjectId>)>) {
        if !self.library_writable {
            curia::warn!("picked structure files refused, the library cannot be saved", { files: files.len() });
            self.notify(ImportNotice::LibraryUnavailable);
            return;
        }
        let mut refused = 0usize;
        for path in files {
            match self.submit(path.clone()) {
                Ok(()) => {
                    if let Some(pick) = pick {
                        self.picked_in_flight.insert(path, pick);
                    }
                }
                Err(ImportBusy) => refused += 1,
            }
        }
        if refused > 0 {
            curia::warn!("picked structure files refused, the import queue is full", { refused: refused });
            self.notify(ImportNotice::ImportQueueFull);
        }
    }

    fn drain_results(&mut self) {
        let Some(worker) = &self.worker else {
            return;
        };
        let mut results = std::mem::take(&mut self.result_buffer);
        results.clear();
        worker.drain(&mut results);
        for result in results.drain(..) {
            self.pending_imports = self.pending_imports.saturating_sub(1);
            match result {
                ImportResult::Ready { source, stored_file, structure } => self.accept(source, stored_file, structure),
                ImportResult::Failed { source, reason } => self.reject(source, reason),
            }
        }
        self.result_buffer = results;
    }

    fn accept(&mut self, source: PathBuf, stored_file: String, mut structure: NewStructure) {
        let from_inbox = self.inbox_in_flight.remove(&source);
        let picked = self.picked_in_flight.remove(&source);
        // A project deleted while the picker was open no longer takes the structure.
        let landed = picked.and_then(|(_, destination)| destination).filter(|id| self.project_exists(*id));
        if picked.is_some() {
            structure.project = landed;
        }
        match self.library.add_structure(structure) {
            Ok(id) => {
                if let Some((serial, _)) = picked
                    && let Some(pick) = self.latest_pick.as_mut().filter(|pick| pick.serial == serial)
                {
                    pick.structures.push(id);
                    pick.destination = landed;
                }
                self.library_changed = true;
                self.changed = true;
                if from_inbox {
                    self.awaiting_save.push(source);
                }
            }
            Err(error) => {
                if let Some(worker) = &self.worker
                    && let Err(e) = worker.discard_stored(&stored_file)
                {
                    curia::error!("refused import could not be removed", {
                        file: stored_file,
                        error: e.to_string(),
                    });
                }
                let file = Self::file_name(&source);
                curia::warn!("structure import failed", { file: file.clone(), reason: error.to_string() });
                // An inbox source stays, and the scanner keeps it marked as listed so it is not offered again.
                self.notify(ImportNotice::ImportRefused { file, error });
            }
        }
    }

    // Removes the inbox sources once the library file that names their structures is on disk.
    fn release_saved_sources(&mut self) {
        if self.awaiting_save.is_empty() {
            return;
        }
        if let Some(writer) = self.store_writer.take() {
            self.release_saved_sources_with(&writer);
            self.store_writer = Some(writer);
        }
    }

    fn release_saved_sources_with(&mut self, writer: &StoreWriter) {
        if self.awaiting_save.is_empty() || self.library.is_dirty() || !writer.saved(&self.library_path) {
            return;
        }
        for source in std::mem::take(&mut self.awaiting_save) {
            self.remove_inbox_source(&source);
        }
    }

    fn remove_inbox_source(&mut self, source: &Path) {
        match std::fs::remove_file(source) {
            Ok(()) => self.inbox.finish(source),
            Err(error) => {
                curia::error!("imported inbox file could not be removed", {
                    path: source.display().to_string(),
                    error: error.to_string(),
                });
                // Not finished: the scanner keeps it marked as listed, so it is not imported twice.
                self.notify(ImportNotice::InboxFileKept { file: Self::file_name(source) });
            }
        }
    }

    fn project_exists(&self, id: ProjectId) -> bool {
        self.library.library().projects.iter().any(|project| project.id == id)
    }

    fn reject(&mut self, source: PathBuf, reason: ImportFailure) {
        self.picked_in_flight.remove(&source);
        if self.inbox_in_flight.remove(&source) {
            self.inbox.reject(&source, &reason);
        }
        let file = Self::file_name(&source);
        curia::warn!("structure import failed", { file: file.clone(), reason: reason.to_string() });
        self.notify(ImportNotice::ImportFailed { file, reason });
    }

    fn delete_structure(&mut self, id: StructureId) {
        match self.library.delete_structure(id) {
            Ok(entry) => {
                self.cache.evict(id);
                self.ghost_loaded.remove(&id);
                self.drop_materials();
                self.remove_stored_file(&entry.file_name);
                self.library_changed = true;
                self.changed = true;
            }
            Err(error) => self.notify(ImportNotice::LibraryRefused(error)),
        }
    }

    fn remove_stored_file(&self, file_name: &str) {
        let plain = Path::new(file_name).file_name().is_some_and(|name| name == file_name);
        if !plain {
            return;
        }
        let path = self.structures_dir().join(file_name);
        if let Err(error) = std::fs::remove_file(&path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            curia::error!("stored structure could not be removed", {
                path: path.display().to_string(),
                error: error.to_string(),
            });
        }
    }

    fn place_marker(&mut self, name: &str, target: MarkerTarget) {
        let now = self.status.time_seconds;
        let repeat =
            self.last_place.is_some_and(|(last, at)| last == target && (now - at) < Self::PLACE_REPEAT_SECONDS);
        if repeat {
            return;
        }
        let origin = self.status.feet;
        let can_place = self.status.in_world && self.status.has_player;
        let Some(markers) = self.markers.as_mut().filter(|_| can_place) else {
            self.notify(ImportNotice::NotInWorld);
            return;
        };
        match markers.place(name, origin, target) {
            Ok(id) => {
                if let Err(error) = markers.activate(id) {
                    curia::error!("placed marker could not be activated", { error: error.to_string() });
                }
                self.last_place = Some((target, now));
                self.changed = true;
            }
            Err(error) => self.notify(ImportNotice::MarkerRefused(error)),
        }
    }

    // What a marker shows: the target name, the layer count, the structure count, the position and
    // the big build layout. `layouts` are the solved
    // big build layouts; a missing or unsolved layout gives no layers. A target that is gone gives
    // no name and no layers.
    fn hud_target<'a>(
        &'a self,
        marker: &Marker,
        layouts: &'a [(ProjectId, Result<BigBuildLayout, LayoutIssue>)],
    ) -> HudTarget<'a> {
        let library = self.library.library();
        let mut target = HudTarget { name: "", layers: 0, structures: 1, position: None, layout: None };
        match marker.target {
            MarkerTarget::Structure(id) => {
                if let Some(entry) = library.structures.iter().find(|entry| entry.id == id) {
                    target.name = &entry.display_name;
                    target.layers = entry.size.y;
                }
            }
            MarkerTarget::Project(project) => match library.projects.iter().find(|entry| entry.id == project) {
                Some(found) if found.kind == ProjectKind::Normal => {
                    let in_project = |entry: &&StructureEntry| entry.project == Some(project);
                    let count = library.structures.iter().filter(in_project).count();
                    let index =
                        usize::try_from(marker.view.selected).unwrap_or(usize::MAX).min(count.saturating_sub(1));
                    let total = u32::try_from(count).unwrap_or(u32::MAX);
                    target.structures = total.max(1);
                    if let Some(entry) = library.structures.iter().filter(in_project).nth(index) {
                        target.name = &entry.display_name;
                        target.layers = entry.size.y;
                        target.position = Some((u32::try_from(index + 1).unwrap_or(u32::MAX), total));
                    }
                }
                Some(found) => {
                    target.name = &found.name;
                    if let Some((_, Ok(layout))) = layouts.iter().find(|(id, _)| *id == project) {
                        target.layers = layout.size.y;
                        target.layout = Some(layout);
                    }
                }
                None => {}
            },
        }
        target
    }

    fn active_is_big_build(&self) -> bool {
        let Some(MarkerTarget::Project(project)) =
            self.markers.as_ref().and_then(MarkerService::active).map(|marker| marker.target)
        else {
            return false;
        };
        self.library.library().projects.iter().any(|entry| entry.id == project && entry.kind == ProjectKind::BigBuild)
    }

    // The HUD card of the active marker. `layouts` are the big build layouts of the snapshot being built.
    fn hud_row(&self, layouts: &[(ProjectId, Result<BigBuildLayout, LayoutIssue>)]) -> Option<HudRow> {
        let marker = self.markers.as_ref().and_then(MarkerService::active)?;
        let HudTarget { name, layers, position, layout, .. } = self.hud_target(marker, layouts);
        let target_name = name.to_owned();
        let piece_name = layout.and_then(|layout| {
            self.nearest_piece(marker.origin, marker.rotation, layout).map(|entry| entry.display_name.clone())
        });
        let layer = marker.view.shown_layer(layers);
        let base = marker.origin[1];
        let top = i32::try_from(layers.saturating_sub(1)).unwrap_or(i32::MAX);
        Some(HudRow {
            marker_name: marker.name.clone(),
            target_name,
            has_structures: position.is_some_and(|(_, count)| count >= 2),
            position,
            piece_name,
            layers,
            layer,
            y_level: layer.map(|layer| base.saturating_add(i32::try_from(layer).unwrap_or(i32::MAX))),
            y_range: (layers > 0).then(|| (base, base.saturating_add(top))),
            shown: marker.view.shown,
            opacity_step: marker.view.opacity().index(),
            origin: marker.origin,
            rotation: marker.rotation,
            easy_place: marker.view.easy_place,
            notice: self.easy_place_notice.as_ref().map(|(_, notice, _)| *notice),
            notice_item: self.easy_place_notice.as_ref().and_then(|(_, _, item)| item.clone()),
        })
    }

    // The piece whose footprint holds the player's feet, else the piece nearest them across the ground.
    // The first piece wins a tie.
    fn nearest_piece(&self, origin: [i32; 3], rotation: Rotation, layout: &BigBuildLayout) -> Option<&StructureEntry> {
        let library = self.library.library();
        let feet = self.status.feet;
        let mut best: Option<(i64, &StructureEntry)> = None;
        for piece in &layout.pieces {
            let Some(entry) = library.structures.iter().find(|entry| entry.id == piece.structure) else {
                continue;
            };
            let offset = piece.offset.map(i64::from);
            let (corner, turned) = rotation.turn_box(offset, [entry.size.x, entry.size.y, entry.size.z]);
            let gap = |axis: usize| {
                let low = i64::from(origin[axis]) + corner[axis];
                let high = low + i64::from(turned[axis]) - 1;
                let at = i64::from(feet[axis]);
                (low - at).max(at - high).max(0)
            };
            let (dx, dz) = (gap(0), gap(2));
            let distance = dx * dx + dz * dz;
            if best.is_none_or(|(nearest, _)| distance < nearest) {
                best = Some((distance, entry));
            }
        }
        best.map(|(_, entry)| entry)
    }

    fn change_markers(&mut self, change: impl FnOnce(&mut MarkerService) -> Result<(), MarkerError>) {
        let Some(markers) = self.markers.as_mut() else {
            self.notify(ImportNotice::NotInWorld);
            return;
        };
        match change(markers) {
            Ok(()) => self.changed = true,
            Err(error) => self.notify(ImportNotice::MarkerRefused(error)),
        }
    }

    fn library_outcome(&mut self, result: Result<(), LibraryError>) {
        match result {
            Ok(()) => {
                self.library_changed = true;
                self.changed = true;
            }
            Err(error) => self.notify(ImportNotice::LibraryRefused(error)),
        }
    }

    // Counts only when asked. A project list is the merge of the lists of its structures.
    // Cached structures are counted now; the others are read by the materials worker and the list
    // arrives on a later tick.
    fn load_materials(&mut self, scope: MaterialScope) {
        self.drop_materials();
        let known = match scope {
            MaterialScope::Structure(id) => self.library.library().structures.iter().any(|entry| entry.id == id),
            MaterialScope::Project(id) => self.library.library().projects.iter().any(|project| project.id == id),
        };
        if !known {
            self.materials_failed = Some(scope);
            self.notify(ImportNotice::LibraryRefused(LibraryError::NotFound));
            return;
        }
        let dir = self.structures_dir();
        let wanted: Vec<(StructureId, PathBuf)> = match scope {
            MaterialScope::Structure(id) => self
                .library
                .library()
                .structures
                .iter()
                .filter(|entry| entry.id == id)
                .map(|entry| (entry.id, dir.join(&entry.file_name)))
                .collect(),
            MaterialScope::Project(id) => self
                .library
                .structures_in(Some(id))
                .into_iter()
                .map(|entry| (entry.id, dir.join(&entry.file_name)))
                .collect(),
        };
        self.materials_job = Some(MaterialsJob { scope, wanted, done: 0, lists: Vec::new(), awaiting: None });
        self.advance_materials();
    }

    // Counts the cached structures of the job in order, and asks the worker for the first one that
    // is not cached. Finishes the job when every structure is counted.
    fn advance_materials(&mut self) {
        let Some(mut job) = self.materials_job.take() else {
            return;
        };
        self.changed = true;
        while let Some((id, path)) = job.wanted.get(job.done).cloned() {
            if let Some(structure) = self.cache.get(id) {
                job.lists.push(MaterialCounter::count(&self.rules, &structure));
                job.done += 1;
                continue;
            }
            let Some(worker) = &self.materials_worker else {
                self.materials_failed = Some(job.scope);
                self.notify(ImportNotice::MaterialsFailed(StructureError::Unreadable(
                    "the structure reader is not available".to_string(),
                )));
                return;
            };
            if worker.submit(StructureLoad { id, path }).is_ok() {
                job.awaiting = Some(id);
                self.materials_job = Some(job);
            } else {
                self.materials_failed = Some(job.scope);
                self.notify(ImportNotice::MaterialsBusy);
            }
            return;
        }
        let lists: Vec<&MaterialList> = job.lists.iter().collect();
        self.materials_failed = None;
        self.materials = Some((job.scope, MaterialList::merge(&lists)));
    }

    // Takes the structures the materials worker finished reading.
    fn drain_loaded(&mut self) {
        let Some(worker) = &self.materials_worker else {
            return;
        };
        let mut loaded = std::mem::take(&mut self.loaded_buffer);
        loaded.clear();
        worker.drain(&mut loaded);
        for answer in loaded.drain(..) {
            self.take_loaded(answer.id, answer.outcome);
        }
        self.loaded_buffer = loaded;
    }

    fn take_loaded(&mut self, id: StructureId, outcome: Result<Arc<Structure>, StructureError>) {
        self.take_ghost_load(id, &outcome);
        let wanted = self.materials_job.as_ref().is_some_and(|job| job.awaiting == Some(id));
        match outcome {
            Ok(structure) => {
                // A structure deleted while it was being read is not cached.
                if self.library.library().structures.iter().any(|entry| entry.id == id) {
                    self.cache.insert(id, Arc::clone(&structure));
                }
                if wanted && let Some(job) = self.materials_job.as_mut() {
                    job.lists.push(MaterialCounter::count(&self.rules, &structure));
                    job.done += 1;
                    job.awaiting = None;
                    self.advance_materials();
                }
            }
            Err(error) if wanted => {
                self.materials_failed = self.materials_job.as_ref().map(|job| job.scope);
                self.materials_job = None;
                self.changed = true;
                self.notify(ImportNotice::MaterialsFailed(error));
            }
            Err(_) => {}
        }
    }

    // Keeps a structure the ghost build needs. An answer to a materials request counts too.
    fn take_ghost_load(&mut self, id: StructureId, outcome: &Result<Arc<Structure>, StructureError>) {
        if !self.ghost.held().contains(&id) {
            return;
        }
        self.ghost_in_flight.remove(&id);
        match outcome {
            Ok(structure) => {
                self.ghost_loaded.insert(id, Arc::clone(structure));
            }
            Err(error) => {
                curia::warn!("ghost structure could not be read", { structure: id.0, error: error.to_string() });
                self.ghost_failed.insert(id);
            }
        }
    }

    // Plans this tick's ghost frame, raises the cut notice and asks for the structures the build
    // still lacks.
    fn update_ghost(&mut self) {
        let marker = self.markers.as_ref().and_then(MarkerService::active);
        let allowed = self.ghost_allowed && marker.is_some_and(|marker| marker.view.shown);
        let started = Instant::now();
        self.ghost.update(
            marker,
            self.library.library(),
            &self.ghost_loaded,
            &self.status,
            allowed,
            &mut self.ghost_frame,
        );
        self.ghost_tick.observe(u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX));
        self.cache.set_charged(self.ghost.turned_bytes());
        self.refresh_place_needs();
        if self.ghost.take_cut_notice() {
            self.notify(ImportNotice::GhostCut);
        }
        if self.ghost.take_refused_notice() {
            self.notify(ImportNotice::GhostRefused);
        }
        self.route_ghost_loads();
    }

    // Builds the placement needs again when the drawn build is a different one. Runs on the tick, never
    // on the use path.
    fn refresh_place_needs(&mut self) {
        let Some((build, _)) = self.ghost.aim_view() else {
            return;
        };
        if self.place_needs.as_ref().is_some_and(|(revision, _)| *revision == build.revision) {
            return;
        }
        self.place_needs = Some((build.revision, PlaceNeeds::new(&build.palette, &self.rules)));
    }

    // Drops what the build no longer needs, takes cached structures, and queues reads for the rest.
    // A full queue is retried on the next tick.
    fn route_ghost_loads(&mut self) {
        let held = self.ghost.held();
        self.ghost_loaded.retain(|id, _| held.contains(id));
        self.ghost_in_flight.retain(|id| held.contains(id));
        self.ghost_failed.retain(|id| held.contains(id));
        let needed = self.ghost.needed();
        for id in needed {
            if self.ghost_loaded.contains_key(id) || self.ghost_in_flight.contains(id) || self.ghost_failed.contains(id)
            {
                continue;
            }
            if let Some(structure) = self.cache.get(*id) {
                self.ghost_loaded.insert(*id, structure);
                continue;
            }
            let Some(entry) = self.library.library().structures.iter().find(|entry| entry.id == *id) else {
                self.ghost_failed.insert(*id);
                continue;
            };
            let Some(worker) = &self.materials_worker else {
                break;
            };
            if worker.submit(StructureLoad { id: *id, path: self.structures_dir.join(&entry.file_name) }).is_err() {
                break;
            }
            self.ghost_in_flight.insert(*id);
        }
    }

    // Forgets the list and any list being built.
    fn drop_materials(&mut self) {
        self.materials = None;
        self.materials_job = None;
        self.materials_failed = None;
        self.changed = true;
    }

    fn notify(&mut self, notice: ImportNotice) {
        if self.notices.len() >= Self::NOTICE_LIMIT {
            self.notices.pop_front();
        }
        self.notices.push_back((self.status.time_seconds, notice));
        self.changed = true;
    }

    fn expire_notices(&mut self) {
        let now = self.status.time_seconds;
        while self.notices.front().is_some_and(|(raised, _)| now - raised >= Self::NOTICE_SECONDS) {
            self.notices.pop_front();
            self.changed = true;
        }
    }

    fn expire_easy_place_notice(&mut self) {
        let now = self.status.time_seconds;
        if self.easy_place_notice.as_ref().is_some_and(|(taken, ..)| now - taken >= EasyPlaceService::NOTICE_SECONDS) {
            self.easy_place_notice = None;
            self.changed = true;
        }
    }

    // Queues the dirty files, at most once per SAVE_INTERVAL_SECONDS. Failed writes are queued again
    // after a wait that doubles with each failure, up to MAX_BACKOFF_SECONDS.
    fn save_if_due(&mut self) {
        let now = self.status.time_seconds;
        if let Some(writer) = &self.store_writer {
            let failures = writer.failures();
            if failures != self.seen_failures {
                self.seen_failures = failures;
                self.backoff_seconds = (self.backoff_seconds * 2.0).min(Self::MAX_BACKOFF_SECONDS);
                self.retry_at = now + self.backoff_seconds;
            } else if !writer.failing() {
                self.backoff_seconds = Self::SAVE_INTERVAL_SECONDS;
            }
            if writer.failed() > 0 && now >= self.retry_at {
                writer.retry_failed();
            }
        }
        let dirty = self.library.is_dirty() || self.markers.as_ref().is_some_and(MarkerService::is_dirty);
        if dirty && now >= self.next_save_at {
            self.next_save_at = now + Self::SAVE_INTERVAL_SECONDS;
            self.save_library();
            self.save_markers();
        }
    }

    // Serialises on this thread and hands the bytes to the store writer.
    fn save_library(&mut self) {
        if !self.library.is_dirty() || !self.library_writable {
            return;
        }
        let Some(writer) = &self.store_writer else {
            return;
        };
        let queued =
            self.library_store.encode(self.library.library()).and_then(|(path, bytes)| writer.submit(path, bytes));
        match queued {
            Ok(()) => self.library.mark_saved(),
            Err(error) => curia::error!("library could not be queued for saving", { error: error.to_string() }),
        }
    }

    fn save_markers(&mut self) {
        let Some(markers) = self.markers.as_mut() else {
            return;
        };
        if !markers.is_dirty() || !self.markers_writable {
            return;
        }
        let Some(writer) = &self.store_writer else {
            return;
        };
        let queued = self.marker_store.encode(markers.set()).and_then(|(path, bytes)| writer.submit(path, bytes));
        match queued {
            Ok(()) => markers.mark_saved(),
            Err(error) => curia::error!("markers could not be queued for saving", { error: error.to_string() }),
        }
    }

    // When the player moved, finds out if the HUD row changed. Only a big build card follows the
    // feet, to name the piece; the marker rows do not depend on the feet.
    fn check_movement(&mut self, moved: bool) {
        if !moved || self.markers.as_ref().is_none_or(|markers| markers.markers().is_empty()) {
            return;
        }
        if self.active_is_big_build() && self.hud_row(&self.snapshot.layouts) != self.snapshot.hud {
            self.changed = true;
        }
    }

    // Builds a new snapshot when something changed. The revision moves only when the content does.
    // The library part is rebuilt only after a library change; otherwise it is copied from the
    // current snapshot.
    fn refresh(&mut self) {
        if !std::mem::take(&mut self.changed) {
            return;
        }
        let library_changed = std::mem::take(&mut self.library_changed);
        if library_changed {
            self.ghost.library_changed();
            self.ghost_failed.clear();
        }
        let rows = self.marker_rows();
        let current = Arc::clone(&self.snapshot);
        let library = library_changed.then(|| self.library_part());
        let layouts = library.as_ref().map_or(&current.layouts, |next| &next.layouts);
        let hud = self.hud_row(layouts);
        if library.is_none() && self.small_parts_match(&current, &rows, &hud) {
            return;
        }
        let mut next = library.unwrap_or_else(|| (*current).clone());
        self.set_small_parts(&mut next, rows, hud);
        if library_changed && next == *current {
            return;
        }
        next.revision = current.revision + 1;
        self.snapshot = Arc::new(next);
    }

    // Projects, structures and big build layouts.
    fn library_part(&self) -> LibrarySnapshot {
        let library = self.library.library();
        let mut projects: Vec<&Project> = library.projects.iter().collect();
        projects.sort_by_cached_key(|project| (project.name.to_lowercase(), project.id));

        let mut rows = Vec::with_capacity(projects.len());
        let mut layouts: Vec<(ProjectId, Result<BigBuildLayout, LayoutIssue>)> = Vec::new();
        for project in projects {
            let pieces = self.library.structures_in(Some(project.id));
            if project.kind == ProjectKind::BigBuild {
                layouts.push((project.id, LayoutSolver::solve(&pieces)));
            }
            rows.push(ProjectRow {
                id: project.id,
                name: project.name.clone(),
                kind: project.kind,
                structures: pieces.into_iter().map(StructureRow::new).collect(),
            });
        }
        LibrarySnapshot {
            projects: rows,
            unsorted: self.library.structures_in(None).into_iter().map(StructureRow::new).collect(),
            all_count: library.structures.len(),
            layouts,
            ..LibrarySnapshot::empty()
        }
    }

    // Everything except the library part and the revision.
    fn set_small_parts(&self, snapshot: &mut LibrarySnapshot, rows: Vec<MarkerRow>, hud: Option<HudRow>) {
        snapshot.markers = rows;
        snapshot.hud = hud;
        snapshot.in_world = self.status.in_world;
        snapshot.has_player = self.status.has_player;
        snapshot.materials.clone_from(&self.materials);
        snapshot.materials_loading = self.materials_job.as_ref().map(|job| job.scope);
        snapshot.materials_failed = self.materials_failed;
        snapshot.notices = self.notices.iter().map(|(_, notice)| notice.clone()).collect();
        snapshot.picker_pending = self.picker_pending || self.picker_open;
        snapshot.latest_pick.clone_from(&self.latest_pick);
        snapshot.library_read_only = !self.library_writable;
        snapshot.markers_read_only = !self.markers_writable;
    }

    fn small_parts_match(&self, snapshot: &LibrarySnapshot, rows: &[MarkerRow], hud: &Option<HudRow>) -> bool {
        snapshot.markers == rows
            && snapshot.hud == *hud
            && snapshot.in_world == self.status.in_world
            && snapshot.has_player == self.status.has_player
            && snapshot.materials == self.materials
            && snapshot.materials_loading == self.materials_job.as_ref().map(|job| job.scope)
            && snapshot.materials_failed == self.materials_failed
            && snapshot.notices.iter().eq(self.notices.iter().map(|(_, notice)| notice))
            && snapshot.picker_pending == (self.picker_pending || self.picker_open)
            && snapshot.latest_pick == self.latest_pick
            && snapshot.library_read_only == !self.library_writable
            && snapshot.markers_read_only == !self.markers_writable
    }

    fn marker_rows(&self) -> Vec<MarkerRow> {
        let Some(markers) = &self.markers else {
            return Vec::new();
        };
        let library = self.library.library();
        let active = markers.set().active;
        markers
            .markers()
            .iter()
            .map(|marker| {
                let target_name = match marker.target {
                    MarkerTarget::Structure(id) => {
                        library.structures.iter().find(|entry| entry.id == id).map(|entry| entry.display_name.clone())
                    }
                    MarkerTarget::Project(id) => {
                        library.projects.iter().find(|project| project.id == id).map(|project| project.name.clone())
                    }
                };
                MarkerRow {
                    id: marker.id,
                    name: marker.name.clone(),
                    target: marker.target,
                    target_name: target_name.unwrap_or_default(),
                    state: markers.target_state(marker.id, library),
                    origin: marker.origin,
                    active: active == Some(marker.id),
                }
            })
            .collect()
    }

    fn structures_dir(&self) -> &Path {
        &self.structures_dir
    }

    // Removes files that no library entry names: copies left by a shutdown before the library recorded them.
    // Runs only at construction, before any import is accepted, so no accepted entry can name a removed file.
    // The inbox source of such a copy is still in the inbox, because a source is removed only once the library
    // file names its structure, so the next scan imports it again.
    fn remove_orphans(data_dir: &Path, library: &Library) {
        let dir = data_dir.join("structures");
        let Ok(entries) = std::fs::read_dir(&dir) else {
            return;
        };
        let known: HashSet<&str> = library.structures.iter().map(|entry| entry.file_name.as_str()).collect();
        for entry in entries.flatten() {
            let is_file = entry.file_type().is_ok_and(|kind| kind.is_file());
            let name = entry.file_name().to_string_lossy().into_owned();
            if is_file && !known.contains(name.as_str()) {
                match std::fs::remove_file(entry.path()) {
                    Ok(()) => curia::info!("orphan structure file removed", { file: name }),
                    Err(error) => curia::error!("orphan structure file could not be removed", {
                        file: name,
                        error: error.to_string(),
                    }),
                }
            }
        }
    }

    fn file_name(path: &Path) -> String {
        path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default()
    }
}

impl Drop for StratumController {
    fn drop(&mut self) {
        self.shutdown();
    }
}
