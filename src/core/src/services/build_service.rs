use std::sync::Arc;

use super::{BuildOrderWorker, BuildSession, LibraryService, MarkerService};
use crate::config::BuildConfig;
use crate::structs::action::BuildAction;
use crate::structs::build::{
    BuildAnchor, BuildAnswer, BuildConditions, BuildKindsRequest, BuildKindsResult, BuildOutbox, BuildPhase,
    BuildProbeRequest, BuildProbeResult, BuildSettingsBook, BuildStopReason,
};
use crate::structs::frame::FrameStatus;
use crate::structs::hud::HudKey;
use crate::structs::library::{Library, ProjectKind, StructureId};
use crate::structs::marker::{Marker, MarkerTarget};
use crate::structs::snapshot::BuildRow;
use crate::structs::structure::Structure;

// Owns the build session, the saved settings of each structure and the commands waiting to be sent. The
// controller calls it each tick with what the game and the other services say, and acts on what it returns:
// requests for the game, commands to send and the row the UI shows. At most one session exists. A session that
// ended stays until it is cancelled or another build is opened.
pub struct BuildService {
    config: BuildConfig,
    book: BuildSettingsBook,
    session: Option<BuildSession>,
    outbox: BuildOutbox,
    conditions: BuildConditions,
    changed: bool,
    // The session row, made when it changes and shared by every reader until the next change.
    shared_row: Option<Arc<BuildRow>>,
    // The session changed since `shared_row` was made.
    row_stale: bool,
    // The world whose restore waits in the outbox, until `take_commands` hands it to the mod.
    restoring: Option<String>,
    // The world whose restore went to the mod in the last `take_commands`.
    restored: Option<String>,
    // The thread that works out the order of a layer; None after `shutdown`.
    worker: Option<BuildOrderWorker>,
    // Layers the worker finished, taken each tick. Bound: `BuildOrderWorker::QUEUE`.
    orders: Vec<BuildAnswer>,
    // The serial the next session starts its jobs from; each session gets its own range.
    serial_base: u64,
    // The time the open session was first seen ended; None while it runs or waits.
    ended_at: Option<f64>,
    // A HUD key came after the end, so the HUD card no longer shows the ended build.
    card_dismissed: bool,
}

impl BuildService {
    // The seconds the HUD card of an ended build stays, unless a HUD key comes first.
    pub const ENDED_CARD_SECONDS: f64 = 10.0;

    pub fn new(config: BuildConfig, book: BuildSettingsBook) -> Self {
        Self::with_worker(config, book, BuildOrderWorker::new())
    }

    pub fn with_worker(config: BuildConfig, book: BuildSettingsBook, worker: BuildOrderWorker) -> Self {
        let outbox = BuildOutbox::new(config.outbox_capacity());
        Self {
            config,
            book,
            session: None,
            outbox,
            conditions: BuildConditions::default(),
            changed: false,
            shared_row: None,
            row_stale: false,
            restoring: None,
            restored: None,
            worker: Some(worker),
            orders: Vec::new(),
            serial_base: 0,
            ended_at: None,
            card_dismissed: false,
        }
    }

    // Stops the order thread and waits for it.
    pub fn shutdown(&mut self) {
        self.worker = None;
    }

    pub fn book(&self) -> &BuildSettingsBook {
        &self.book
    }

    pub fn is_dirty(&self) -> bool {
        self.book.is_dirty()
    }

    pub fn mark_saved(&mut self) {
        self.book.mark_saved();
    }

    // True once after the row or the allowed structure may have changed.
    pub fn take_changed(&mut self) -> bool {
        std::mem::take(&mut self.changed)
    }

    // What the game says about the world and the player now. Call it each tick before `tick`.
    pub fn set_conditions(&mut self, conditions: BuildConditions) {
        if self.conditions != conditions {
            self.conditions = conditions;
            self.touch();
        }
    }

    // The structure whose page may show Build: every condition holds, and the active marker shows the structure
    // and is not a big build marker.
    pub fn allowed(&self, markers: Option<&MarkerService>, library: &LibraryService) -> Option<StructureId> {
        if !self.conditions.allowed() {
            return None;
        }
        Self::shown(markers?.active()?, library.library())
    }

    // The structure of the open build, if there is one.
    pub fn structure(&self) -> Option<StructureId> {
        self.session.as_ref().map(BuildSession::structure)
    }

    // The panel or card row; None when no build is open.
    pub fn row(&self) -> Option<BuildRow> {
        self.session.as_ref().map(BuildSession::row)
    }

    // The row of the HUD card at `now`, made once for each change: the session row, except that an ended build leaves it after
    // `ENDED_CARD_SECONDS` or at the first HUD key after the end. The library snapshot keeps the ended row.
    pub fn hud_row(&self, now: f64) -> Option<Arc<BuildRow>> {
        let session = self.session.as_ref()?;
        if session.is_finished() {
            let expired = self.ended_at.is_some_and(|at| now - at >= Self::ENDED_CARD_SECONDS);
            if self.card_dismissed || expired {
                return None;
            }
        }
        self.shared_row.clone()
    }

    // Marks the row and the snapshot as changed.
    fn touch(&mut self) {
        self.changed = true;
        self.row_stale = true;
    }

    // Keeps the book of owed restores in step with the session: a world is owed the restore from the first command of
    // a build until the restore command leaves the outbox, so a close or a crash in between leaves it on disk.
    fn track_restore(&mut self) {
        let Some(session) = self.session.as_mut() else {
            return;
        };
        let queued = session.take_restore_queued();
        let Some(anchor) = session.anchor() else {
            return;
        };
        if session.rule_changed() {
            self.book.owe(&anchor.world);
        }
        if queued {
            self.restoring = Some(anchor.world.clone());
        }
    }

    // Sends the restore a world is owed from an earlier build that ended with the world gone. It goes once, when the
    // player is in that world, commands work there (creative is not needed), and no build runs.
    fn send_owed_restore(&mut self, status: &FrameStatus) {
        let conditions = &self.conditions;
        let allowed = status.in_world && conditions.local && conditions.cheats && conditions.operator;
        if !allowed || self.restoring.is_some() || self.is_running() || !self.book.owes(&status.world) {
            return;
        }
        if self.outbox.push(BuildSession::restore_command()).is_ok() {
            self.restoring = Some(status.world.clone());
        }
    }

    // Makes the shared row again when the session changed since it was made. Called at the end of every method that
    // changes the session, so a frame reads the row without making one.
    fn refresh_row(&mut self) {
        self.track_restore();
        if std::mem::take(&mut self.row_stale) {
            self.shared_row = self.session.as_ref().map(|session| Arc::new(session.row()));
        }
    }

    // Remembers when the session ended, and forgets it for a session that has not.
    fn note_end(&mut self, now: f64) {
        if self.session.as_ref().is_some_and(BuildSession::is_finished) {
            self.ended_at.get_or_insert(now);
        } else {
            self.ended_at = None;
            self.card_dismissed = false;
        }
    }

    // True while the marker must not change.
    pub fn locks_marker(&self) -> bool {
        self.session.as_ref().is_some_and(BuildSession::locks_marker)
    }

    // True while a build runs, so the HUD keys act on it.
    pub fn is_running(&self) -> bool {
        self.session.as_ref().is_some_and(|session| session.phase().is_running())
    }

    // Applies an action of the UI or the HUD. `now` is the time of the last frame.
    pub fn apply(&mut self, action: BuildAction, now: f64, markers: Option<&MarkerService>, library: &LibraryService) {
        match action {
            BuildAction::Open(id) => self.open(id, markers, library),
            BuildAction::SetSettings(id, settings) => {
                let settings = settings.clamped();
                self.book.set(id, settings);
                if let Some(session) = self.session.as_mut().filter(|session| session.structure() == id) {
                    session.set_settings(settings);
                }
            }
            BuildAction::Start => {
                if self.conditions.allowed()
                    && let Some(session) = self.session.as_mut()
                {
                    session.start(now, &mut self.outbox);
                }
            }
            BuildAction::Pause => {
                if let Some(session) = self.session.as_mut() {
                    session.pause(now);
                }
            }
            BuildAction::Resume => {
                if let Some(session) = self.session.as_mut() {
                    session.resume(now);
                }
            }
            BuildAction::Stop => {
                if let Some(session) = self.session.as_mut() {
                    session.stop(BuildStopReason::Operator, &mut self.outbox);
                }
            }
            BuildAction::Cancel => {
                if self.session.as_ref().is_some_and(|session| !session.phase().is_running()) {
                    self.session = None;
                }
            }
        }
        self.pump();
        self.note_end(now);
        self.touch();
        self.refresh_row();
    }

    // One HUD key while a build runs: Up pauses or resumes, Down asks for a second press to stop. True when the
    // key was used.
    pub fn hud_key(&mut self, key: HudKey, now: f64) -> bool {
        let used = self.press_key(key, now);
        self.refresh_row();
        used
    }

    fn press_key(&mut self, key: HudKey, now: f64) -> bool {
        if self.session.as_ref().is_some_and(BuildSession::is_finished) {
            // The first key after the end takes the ended card away and is not used: the marker keys get it.
            self.note_end(now);
            if !self.card_dismissed {
                self.card_dismissed = true;
                self.touch();
            }
            return false;
        }
        let Some(session) = self.session.as_mut().filter(|session| session.phase().is_running()) else {
            return false;
        };
        match key {
            HudKey::Up if session.phase() == BuildPhase::Paused => session.resume(now),
            HudKey::Up => session.pause(now),
            HudKey::Down => {
                session.press_stop(now, &mut self.outbox);
            }
            _ => return false,
        }
        self.note_end(now);
        self.touch();
        true
    }

    // The structure the open build waits to have loaded, until `tick` gets it.
    pub fn wants_structure(&self) -> Option<StructureId> {
        self.session.as_ref().filter(|session| session.wants_build()).map(BuildSession::structure)
    }

    // The structure could not be read: the panel closes.
    pub fn structure_unreadable(&mut self) {
        if self.session.as_ref().is_some_and(BuildSession::wants_build) {
            self.session = None;
            self.touch();
            self.refresh_row();
        }
    }

    // Checks the stop conditions, makes the build of the structure from the active marker when the session waits
    // for it, takes the layers the worker finished, and sends the commands that are due. `structure` is the loaded
    // structure of the session. The build never depends on the ghost: it is made from the marker as it is now,
    // and the marker is held still from the start.
    pub fn tick(
        &mut self,
        status: &FrameStatus,
        structure: Option<&Arc<Structure>>,
        markers: Option<&MarkerService>,
        library: &LibraryService,
    ) {
        self.advance(status, structure, markers, library);
        self.refresh_row();
    }

    fn advance(
        &mut self,
        status: &FrameStatus,
        structure: Option<&Arc<Structure>>,
        markers: Option<&MarkerService>,
        library: &LibraryService,
    ) {
        self.note_end(status.time_seconds);
        self.send_owed_restore(status);
        let Some(session) = self.session.as_mut() else {
            return;
        };
        if session.is_finished() {
            if !status.in_world {
                self.session = None;
                self.touch();
            }
            return;
        }
        let active = markers.and_then(MarkerService::active);
        if let Some(reason) = Self::stop_reason(session, &self.conditions, status, active, library.library()) {
            if session.phase().is_running() {
                session.stop(reason, &mut self.outbox);
            } else {
                self.session = None;
            }
            self.touch();
            return;
        }
        if session.wants_build()
            && let (Some(structure), Some(marker)) = (structure, active)
            && Self::shown(marker, library.library()) == Some(session.structure())
        {
            let anchor = BuildAnchor {
                world: status.world.clone(),
                dimension: status.dimension,
                marker: marker.id,
                origin: marker.origin,
                rotation: marker.rotation,
                structure: session.structure(),
            };
            session.begin_assemble(anchor, Arc::clone(structure), marker.origin, marker.rotation);
            self.touch();
        }
        if let (Some(worker), Some(session)) = (self.worker.as_ref(), self.session.as_mut()) {
            worker.drain(&mut self.orders);
            for answer in self.orders.drain(..) {
                session.apply_answer(answer);
                self.changed = true;
                self.row_stale = true;
            }
        }
        if self.session.as_ref().is_some_and(BuildSession::failed) {
            self.session = None;
            self.touch();
            return;
        }
        if let Some(session) = self.session.as_mut()
            && session.tick(status.time_seconds, &mut self.outbox)
        {
            self.touch();
        }
        self.pump();
    }

    // Hands the layer the session wants worked out to the worker.
    fn pump(&mut self) {
        let (Some(session), Some(worker)) = (self.session.as_mut(), self.worker.as_ref()) else {
            return;
        };
        if let Some(job) = session.take_job()
            && let Err(job) = worker.submit(job)
        {
            session.release_job(job);
        }
    }

    // The palette request for the game, once.
    pub fn take_probe(&mut self) -> Option<BuildProbeRequest> {
        self.session.as_mut()?.take_probe()
    }

    pub fn apply_probe(&mut self, result: &BuildProbeResult) {
        if let Some(session) = self.session.as_mut() {
            session.apply_probe(result);
            self.touch();
        }
        self.pump();
        self.refresh_row();
    }

    // The block kinds request for the game, once.
    pub fn take_kinds(&mut self) -> Option<BuildKindsRequest> {
        self.session.as_mut()?.take_kinds()
    }

    pub fn apply_kinds(&mut self, result: BuildKindsResult, now: f64) {
        if let Some(session) = self.session.as_mut() {
            session.apply_kinds(result, now, &mut self.outbox);
            self.touch();
        }
        self.refresh_row();
    }

    // Moves the commands that wait to be sent to `out`, oldest first.
    pub fn take_commands(&mut self, out: &mut Vec<String>) {
        let before = out.len();
        self.outbox.drain(out);
        // The restore left the outbox: the world is no longer owed it, unless the mod reports a failed send.
        self.restored = if out.len() > before { self.restoring.take() } else { None };
        if let Some(world) = &self.restored {
            self.book.clear_owed(world);
        }
    }

    // The mod could not send a command: the running build stops.
    pub fn send_failed(&mut self) {
        // The failed batch held the restore of an ended build: the world is owed it again.
        if let Some(world) = self.restored.take() {
            self.book.owe(&world);
        }
        if let Some(session) = self.session.as_mut() {
            session.stop(BuildStopReason::SendFailed, &mut self.outbox);
            self.touch();
        }
        self.refresh_row();
    }

    // The library deleted a structure: its settings go.
    pub fn structure_deleted(&mut self, id: StructureId) {
        self.book.remove(id);
    }

    fn open(&mut self, id: StructureId, markers: Option<&MarkerService>, library: &LibraryService) {
        if self.is_running() || self.allowed(markers, library) != Some(id) {
            return;
        }
        let Some(entry) = library.library().structures.iter().find(|entry| entry.id == id) else {
            return;
        };
        let settings = self.book.use_settings(id);
        let mut session = BuildSession::new(self.config.kinds_capacity(), id, entry.display_name.clone(), settings);
        session.start_serials(self.serial_base);
        self.serial_base += 1 << 20;
        self.session = Some(session);
    }

    // Why the build must end now, if it must. Leaving the world is named first, then a changed dimension, a
    // deleted structure, a lost condition and last a changed marker.
    fn stop_reason(
        session: &BuildSession,
        conditions: &BuildConditions,
        status: &FrameStatus,
        active: Option<&Marker>,
        library: &Library,
    ) -> Option<BuildStopReason> {
        let anchor = session.anchor();
        let other_world = anchor.is_some_and(|anchor| anchor.world != status.world);
        if !status.in_world || other_world {
            return Some(BuildStopReason::LeftWorld);
        }
        if anchor.is_some_and(|anchor| anchor.dimension != status.dimension) {
            return Some(BuildStopReason::DimensionChanged);
        }
        if !library.structures.iter().any(|entry| entry.id == session.structure()) {
            return Some(BuildStopReason::StructureDeleted);
        }
        if !conditions.allowed() {
            return Some(BuildStopReason::ConditionLost);
        }
        let anchor = anchor?;
        let unchanged = active.is_some_and(|marker| {
            marker.id == anchor.marker
                && marker.origin == anchor.origin
                && marker.rotation == anchor.rotation
                && Self::shown(marker, library) == Some(anchor.structure)
        });
        if unchanged { None } else { Some(BuildStopReason::MarkerChanged) }
    }

    // The structure a marker shows: its structure, or the current structure of a normal project. A big build shows
    // none.
    fn shown(marker: &Marker, library: &Library) -> Option<StructureId> {
        let exists = |id: StructureId| library.structures.iter().any(|entry| entry.id == id);
        match marker.target {
            MarkerTarget::Structure(id) => exists(id).then_some(id),
            MarkerTarget::Project(project) => {
                let normal =
                    library.projects.iter().any(|entry| entry.id == project && entry.kind == ProjectKind::Normal);
                marker.view.current.filter(|id| normal && exists(*id))
            }
        }
    }
}
