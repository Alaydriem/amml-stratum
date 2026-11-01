use std::collections::HashMap;
use std::sync::Arc;

use crate::config::GhostConfig;
use crate::errors::GhostError;
use crate::services::{GhostAssembler, GhostScheduler, GhostSections, GhostTurnWorker, SectionScan};
use crate::structs::frame::FrameStatus;
use crate::structs::ghost::source::BuildSource;
use crate::structs::ghost::{GhostBuild, GhostFrame, GhostReport, GhostView, MiniatureGroups, SectionKey, SectionPart};
use crate::structs::ghost::{PlacedStructure, TurnDone, TurnJob};
use crate::structs::library::{Library, StructureId};
use crate::structs::marker::{Marker, MarkerView, OpacityStep, Rotation};
use crate::structs::structure::Structure;

// Turns the active marker into a ghost frame once per tick: loads nothing itself, but names the
// structures it needs, assembles the build when they are there, picks the view, plans the mesh
// slots and keeps the sections that a world block change touched up to date.
//
// Bound: `changes` holds at most `BLOCK_CHANGE_CAPACITY` positions and is emptied by every
// update that shows the ghost; the positions past it are kept as one box in `dropped`.
// `miniature_groups` holds at most `slot_capacity / MINIATURE_SLOT_DIVISOR` sections, because a build
// with more occupied sections gets none, and is emptied when the build goes or is replaced.
// `occupied`, `occupied_cells`, `rescanned`, `wanted` and `needed` are bounded by the active build, which is bounded by the
// palette and layout limits, and are emptied when the build goes. `placed` holds one entry per
// piece of the active build and is emptied when the build goes. `turned` holds one turned copy of
// each piece of the active build, only while the build is turned; it goes when the build turns back
// to R0, when the marker names other structures and when the world is left. `turned_bytes` gives its
// size, which the controller charges to the structure cache budget. At most one turn is in flight
// on the worker. `keys` holds at most
// `BLOCK_CHANGE_CAPACITY * KEYS_PER_POSITION` keys, sorted and deduplicated before use.
pub struct GhostService {
    config: GhostConfig,
    scheduler: GhostScheduler,
    // The generation of the next build or world; it moves on every build change and world change.
    revision: u64,
    world: Option<(String, i32)>,
    // Moves on every world change, including leaving a world.
    world_epoch: u64,
    source: Option<BuildSource>,
    needed: Vec<StructureId>,
    // False while the ghost may not draw: `needed` reads as empty then.
    shown: bool,
    // `needed` is set and the build waits for the structures.
    awaiting: bool,
    library_dirty: bool,
    build: Option<GhostBuild>,
    // The structures of `build` with their unturned positions, and the cell and turn they were
    // turned around. A new assembly that matches them keeps the build.
    placed: Vec<PlacedStructure>,
    pivot: [i32; 3],
    rotation: Rotation,
    // The marker moved `shift` blocks from where `build` sits. The meshes are drawn moved by it,
    // and the build is made at the marker's place once no move came until `settle_at`.
    shift: [i32; 3],
    settle_at: f64,
    // A HUD key is down: a shifted build does not settle until it is released.
    keys_held: bool,
    // The keys were released since the last update, which starts the quiet time again.
    keys_released: bool,
    // True until a waiting shift reads the layout after a change of the source or the library, or
    // after a build that has not run yet.
    reassemble: bool,
    // The opacity step the meshes are built with.
    opacity: OpacityStep,
    // The palette goes out again in the next plan, so the shim turns its block states again.
    resend: bool,
    turner: GhostTurnWorker,
    // The turned copies of the active build's structures.
    turned: Option<TurnDone>,
    // The ticket of the turn on the worker, and the last ticket given out.
    turning: Option<u64>,
    ticket: u64,
    // The worker was found gone; logged once, and turns run on this thread from then on.
    turner_lost: bool,
    occupied: Vec<SectionKey>,
    // The filled cell count of each key of `occupied`, in the same order.
    occupied_cells: Vec<u32>,
    // The scan that is still filling `occupied` and `occupied_cells`, or `rescanned`.
    scan: Option<SectionScan>,
    // Filled by a scan that a settings change started while `occupied` is complete. `occupied` keeps feeding
    // `wanted` until the scan is done, then this replaces it, so the built meshes stay drawn meanwhile.
    rescanned: Option<(Vec<SectionKey>, Vec<u32>)>,
    wanted: Vec<SectionKey>,
    // The groups of occupied keys the mini ghost merges into one mesh each; empty while the section
    // scan runs, when the switch is off and when the build has more sections than the slot limit.
    miniature_groups: MiniatureGroups,
    // The mini ghost is on, the scan is done and the build has more sections than the slot limit.
    miniature_over_limit: bool,
    // The view the current slots were planned for.
    view: Option<GhostView>,
    changes: Vec<[i32; 3]>,
    // The inclusive box of the block changes that did not fit in `changes`.
    dropped: Option<([i32; 3], [i32; 3])>,
    keys: Vec<SectionKey>,
    overflow: bool,
    overflows: u64,
    cut: bool,
    cut_noticed: bool,
    cut_notice: bool,
    refused_notice: bool,
    failed_slots: u64,
    unknown_blocks: u64,
    default_state_blocks: u64,
}

impl GhostService {
    // Block change positions kept between two updates.
    pub const BLOCK_CHANGE_CAPACITY: usize = 4096;
    // Sections a tick reads while a new build is scanned: 16 * 4096 = 65,536 cell lookups.
    pub const SCAN_SECTIONS_PER_TICK: usize = 16;
    // Most keys one block change position can mark: 2 sections on each of x and z, a Whole and a
    // Lower or Upper part on y, and 4 Layer keys.
    pub const KEYS_PER_POSITION: usize = 20;
    // Quiet time after the last move of the marker before the moved build is made at its new place.
    pub const MOVE_SETTLE_SECONDS: f64 = 0.3;
    // The mini ghost is made for at most this fraction of the slot table in sections: one in `MINIATURE_SLOT_DIVISOR`.
    pub const MINIATURE_SLOT_DIVISOR: u32 = 4;

    pub fn new(config: GhostConfig) -> Self {
        Self::with_turn_worker(config, GhostTurnWorker::new())
    }

    // A worker without a thread turns builds on the caller's thread.
    pub fn with_turn_worker(config: GhostConfig, turner: GhostTurnWorker) -> Self {
        Self {
            scheduler: GhostScheduler::new(&config),
            config,
            revision: 0,
            world: None,
            world_epoch: 0,
            source: None,
            needed: Vec::new(),
            shown: false,
            awaiting: false,
            library_dirty: false,
            build: None,
            placed: Vec::new(),
            pivot: [0; 3],
            rotation: Rotation::R0,
            shift: [0; 3],
            settle_at: 0.0,
            keys_held: false,
            keys_released: false,
            reassemble: false,
            opacity: MarkerView::default().opacity(),
            resend: false,
            turner,
            turned: None,
            turning: None,
            ticket: 0,
            turner_lost: false,
            occupied: Vec::new(),
            occupied_cells: Vec::new(),
            scan: None,
            rescanned: None,
            wanted: Vec::new(),
            miniature_groups: MiniatureGroups::new(),
            miniature_over_limit: false,
            view: None,
            changes: Vec::with_capacity(Self::BLOCK_CHANGE_CAPACITY),
            dropped: None,
            keys: Vec::with_capacity(Self::BLOCK_CHANGE_CAPACITY * Self::KEYS_PER_POSITION),
            overflow: false,
            overflows: 0,
            cut: false,
            cut_noticed: false,
            cut_notice: false,
            refused_notice: false,
            failed_slots: 0,
            unknown_blocks: 0,
            default_state_blocks: 0,
        }
    }

    // Takes the settings that apply while the game runs: draw distance, cells per
    // frame and the mini ghost. The mesh budget sizes the slot table and stays as it was until a restart. A new frame
    // allowance scans the build again, because it decides where a section splits. The old sections
    // stay in use until the new scan is done, so only the keys whose split changed are built again.
    pub fn reconfigure(&mut self, config: GhostConfig) {
        let config = GhostConfig { mesh_budget_mb: self.config.mesh_budget_mb, ..config };
        if config == self.config {
            return;
        }
        let rescan = config.cells_per_frame() != self.config.cells_per_frame();
        self.scheduler.set_cells_per_frame(config.cells_per_frame());
        self.config = config;
        if rescan && let Some(build) = self.build.as_ref() {
            if self.scan.is_some() && self.rescanned.is_none() {
                self.occupied.clear();
                self.occupied_cells.clear();
            } else {
                self.rescanned = Some((Vec::new(), Vec::new()));
            }
            self.scan = Some(SectionScan::new(build));
        }
    }

    // True while a HUD key is down. A shifted build settles `MOVE_SETTLE_SECONDS` after the later of
    // the last move and the release.
    pub fn set_keys_held(&mut self, held: bool) {
        self.keys_released |= self.keys_held && !held;
        self.keys_held = held;
    }

    // The settings in use.
    pub fn config(&self) -> &GhostConfig {
        &self.config
    }

    // The structures the controller must load for the active build.
    pub fn needed(&self) -> &[StructureId] {
        if self.shown { &self.needed } else { &[] }
    }

    // Every structure the current build or pending build is made of, also while the ghost is hidden.
    pub fn held(&self) -> &[StructureId] {
        &self.needed
    }

    // The build and the view the ghost draws now. None while the ghost is hidden, while the build
    // is not ready and while a move has not settled, because the build then sits where the marker was.
    pub fn aim_view(&self) -> Option<(&GhostBuild, GhostView)> {
        if !self.shown || self.awaiting || self.shift != [0; 3] {
            return None;
        }
        Some((self.build.as_ref()?, self.view?))
    }

    // True while a new build is still being scanned for its sections.
    pub fn scanning(&self) -> bool {
        self.scan.is_some()
    }

    // Tells the service that the library changed. The build is checked again on the next update.
    pub fn library_changed(&mut self) {
        self.library_dirty = true;
    }

    // The shim must give its meshes back (the world draw lease was revoked): the next update clears
    // every slot, and the next show builds them again.
    pub fn drop_slots(&mut self) {
        self.start_generation();
    }

    // Records a world block change. A position is kept until the next update that shows the ghost;
    // past `BLOCK_CHANGE_CAPACITY` the positions are kept only as one box, and the sections that
    // box touches are marked stale. While the build is drawn moved, a position marks the sections
    // of the build's own place, because their meshes skip the world blocks of that place.
    pub fn block_changed(&mut self, position: [i32; 3]) {
        if self.build.is_none() {
            return;
        }
        if self.changes.len() < Self::BLOCK_CHANGE_CAPACITY {
            self.changes.push(position);
            return;
        }
        if !self.overflow {
            self.overflow = true;
            self.overflows += 1;
        }
        self.dropped = Some(match self.dropped {
            Some((low, high)) => (
                [0, 1, 2].map(|axis| low[axis].min(position[axis])),
                [0, 1, 2].map(|axis| high[axis].max(position[axis])),
            ),
            None => (position, position),
        });
    }

    // True once when a generation first lost a section to the mesh budget.
    pub fn take_cut_notice(&mut self) -> bool {
        std::mem::take(&mut self.cut_notice)
    }

    // True once when a build was refused because its palette is too large.
    pub fn take_refused_notice(&mut self) -> bool {
        std::mem::take(&mut self.refused_notice)
    }

    // The shim's answers for the jobs it ran.
    pub fn apply_report(&mut self, report: &GhostReport) {
        self.scheduler.apply_report(report);
        self.failed_slots += report.built.iter().filter(|built| !built.ok).count() as u64;
        if report.palette_resolved {
            self.unknown_blocks = u64::from(report.unknown_blocks);
            self.default_state_blocks = u64::from(report.default_state_blocks);
        }
        if report.world_reset {
            self.start_generation();
        }
    }

    pub fn live_slots(&self) -> u32 {
        self.scheduler.live_slots()
    }

    pub fn budget_bytes(&self) -> u64 {
        self.config.mesh_budget_bytes()
    }

    pub fn slot_capacity(&self) -> u32 {
        self.scheduler.slot_capacity()
    }

    pub fn used_bytes(&self) -> u64 {
        self.scheduler.used_bytes()
    }

    // Finished mini ghost groups.
    pub fn miniature_slots(&self) -> u32 {
        self.scheduler.miniature_group_slots(&self.miniature_groups).0
    }

    // Sections built into finished mini ghost groups.
    pub fn miniature_sections(&self) -> u32 {
        self.scheduler.miniature_group_slots(&self.miniature_groups).1
    }

    // Reported bytes of the finished mini ghost groups.
    pub fn miniature_bytes(&self) -> u64 {
        self.scheduler.miniature_group_slots(&self.miniature_groups).2
    }

    // True when the mini ghost is on but the build has more sections than its slot limit allows.
    pub fn miniature_over_limit(&self) -> bool {
        self.miniature_over_limit
    }

    // True when a wanted section went without a slot in the current generation.
    pub fn cut(&self) -> bool {
        self.cut
    }

    pub fn block_change_overflows(&self) -> u64 {
        self.overflows
    }

    pub fn failed_slots(&self) -> u64 {
        self.failed_slots
    }

    // The fallbacks of the last palette the shim resolved.
    pub fn unknown_blocks(&self) -> u64 {
        self.unknown_blocks
    }

    pub fn default_state_blocks(&self) -> u64 {
        self.default_state_blocks
    }

    // Fills `frame` for this tick. `allowed` is false when the world draw lease is not held or the
    // marker is hidden: the frame then draws nothing, and the build and its slots are kept.
    pub fn update(
        &mut self,
        marker: Option<&Marker>,
        library: &Library,
        loaded: &HashMap<StructureId, Arc<Structure>>,
        status: &FrameStatus,
        allowed: bool,
        frame: &mut GhostFrame,
    ) {
        self.track_world(status);
        frame.world_epoch = self.world_epoch;
        frame.in_world = self.world.is_some();
        self.shown = marker.is_some() && allowed;
        let Some(marker) = marker else {
            self.forget_source();
            self.scheduler.plan(None, &[], &self.miniature_groups, [0; 3], frame);
            frame.opacity = self.opacity.alpha();
            frame.rotation = self.rotation;
            return;
        };
        frame.opacity = marker.view.opacity().alpha();
        if !allowed {
            self.hold(marker, frame);
            frame.rotation = self.rotation;
            return;
        }

        self.refresh_source(marker, library, loaded);
        self.assemble(marker, library, loaded, status.time_seconds);
        self.apply_changes();
        frame.rotation = self.rotation;

        let Some(build) = self.build.as_ref() else {
            self.scheduler.plan(None, &[], &self.miniature_groups, [0; 3], frame);
            return;
        };
        if marker.view.opacity() != self.opacity {
            self.opacity = marker.view.opacity();
            self.scheduler.invalidate_all();
        }
        let on = self.config.miniature;
        let limit = self.miniature_limit();
        if let Some(scan) = self.scan.as_mut() {
            let (out, cells) = match self.rescanned.as_mut() {
                Some((keys, counts)) => (keys, counts),
                None => (&mut self.occupied, &mut self.occupied_cells),
            };
            if scan.step(build, self.config.cells_per_frame(), Self::SCAN_SECTIONS_PER_TICK, out, cells) {
                self.scan = None;
                if let Some((keys, counts)) = self.rescanned.take() {
                    self.occupied = keys;
                    self.occupied_cells = counts;
                }
                Self::recut_miniature(
                    on,
                    limit,
                    &self.occupied,
                    &self.occupied_cells,
                    &mut self.miniature_groups,
                    &mut self.scheduler,
                );
            }
        }
        if self.scan.is_none() {
            self.miniature_over_limit = on && self.occupied.len() > limit;
            if !on || self.miniature_over_limit {
                self.miniature_groups.clear();
            } else if self.miniature_groups.group_count() == 0 && !self.occupied.is_empty() {
                Self::recut_miniature(
                    on,
                    limit,
                    &self.occupied,
                    &self.occupied_cells,
                    &mut self.miniature_groups,
                    &mut self.scheduler,
                );
            }
        }
        let view = Self::marker_view(&marker.view, build);
        if self.view.is_some_and(|planned| planned != view) {
            self.scheduler.hold_unwanted();
        }
        self.view = Some(view);
        self.scheduler.set_wanted_complete(self.scan.is_none() || self.rescanned.is_some());
        let feet = [0, 1, 2].map(|axis| status.feet[axis].wrapping_sub(self.shift[axis]));
        GhostSections::wanted(
            &self.occupied,
            view,
            feet,
            self.config.draw_distance_chunks(status.render_distance),
            &mut self.wanted,
        );
        Self::add_miniature(view, &self.miniature_groups, &mut self.wanted, &mut self.scheduler);
        self.scheduler.plan(Some(build), &self.wanted, &self.miniature_groups, self.shift, frame);
        if std::mem::take(&mut self.resend) && frame.palette.is_none() {
            frame.palette = Some(Arc::new(build.palette.clone()));
        }
        if frame.cut {
            self.cut = true;
            if !self.cut_noticed {
                self.cut_noticed = true;
                self.cut_notice = true;
            }
        }
    }

    // Largest number of occupied sections the mini ghost is made for.
    fn miniature_limit(&self) -> usize {
        (self.scheduler.slot_capacity() / Self::MINIATURE_SLOT_DIVISOR) as usize
    }

    // Cuts the groups from `occupied` when the mini ghost is on and fits its slot limit, else empties
    // them. Groups that were in use and come out different are all built again.
    fn recut_miniature(
        on: bool,
        limit: usize,
        occupied: &[SectionKey],
        cells: &[u32],
        groups: &mut MiniatureGroups,
        scheduler: &mut GhostScheduler,
    ) {
        if !on || occupied.len() > limit {
            groups.clear();
            return;
        }
        let before = groups.clone();
        groups.cut(occupied, cells);
        if before.group_count() > 0 && before != *groups {
            let keys: Vec<SectionKey> = before.keys().chain(groups.keys()).collect();
            scheduler.invalidate(&keys);
        }
    }

    // Puts the mini ghost and the marker in front of the full keys of `wanted`. Whole view draws the
    // mini ghost and no marker; Layer view draws the marker and keeps the mini ghost slots built but
    // undrawn; without a mini ghost the marker is drawn in every view.
    fn add_miniature(
        view: GhostView,
        groups: &MiniatureGroups,
        wanted: &mut Vec<SectionKey>,
        scheduler: &mut GhostScheduler,
    ) {
        let marker = [GhostSections::MARKER_KEY];
        let count = groups.group_count();
        let keys = groups.keys();
        if count == 0 {
            wanted.splice(0..0, marker);
            scheduler.set_undrawn_lead(0);
        } else if matches!(view, GhostView::Layer(_)) {
            wanted.splice(0..0, keys.chain(marker));
            scheduler.set_undrawn_lead(count as usize);
        } else {
            wanted.splice(0..0, keys);
            scheduler.set_undrawn_lead(0);
        }
    }

    // The view the marker asks for: one layer counted from the build's lowest Y, or the whole build
    // when the view is Whole or the layer is past the top.
    fn marker_view(view: &MarkerView, build: &GhostBuild) -> GhostView {
        let layers = i64::from(build.world_max[1]) - i64::from(build.world_min[1]) + 1;
        let layers = u32::try_from(layers).unwrap_or(u32::MAX);
        match view.shown_layer(layers).and_then(|layer| i32::try_from(layer).ok()) {
            Some(layer) => GhostView::Layer(build.world_min[1].saturating_add(layer)),
            None => GhostView::Whole,
        }
    }

    // Hidden, or the lease is not held: nothing is drawn and the slots stay for the next show. A
    // different marker drops the build; dropped slots, or a build that went, are cleared. A move or
    // a turn waits for the next show.
    fn hold(&mut self, marker: &Marker, frame: &mut GhostFrame) {
        let same = self.source.as_ref().is_some_and(|source| source.same_blocks(&BuildSource::new(marker)));
        if !same {
            self.forget_source();
        }
        let revision = self.build.as_ref().map(|build| build.revision);
        if revision.is_some() && revision == self.scheduler.generation() {
            self.scheduler.pause(frame);
        } else {
            self.scheduler.plan(None, &[], &self.miniature_groups, [0; 3], frame);
        }
    }

    // A new generation: the shim gets the palette and clears its slots again.
    fn start_generation(&mut self) {
        self.revision += 1;
        if let Some(build) = self.build.as_mut() {
            build.revision = self.revision;
        }
        self.cut = false;
    }

    // A new world, or none, starts a new generation so the shim resolves its blocks again.
    fn track_world(&mut self, status: &FrameStatus) {
        let same = match &self.world {
            Some((world, dimension)) => status.in_world && *world == status.world && *dimension == status.dimension,
            None => !status.in_world,
        };
        if same {
            return;
        }
        self.forget_source();
        self.revision += 1;
        self.world_epoch += 1;
        self.world = status.in_world.then(|| (status.world.clone(), status.dimension));
    }

    // Drops the build and everything derived from the marker.
    fn forget_source(&mut self) {
        self.drop_build();
        self.source = None;
        self.needed.clear();
        self.awaiting = false;
        self.library_dirty = false;
    }

    fn drop_build(&mut self) {
        if self.build.take().is_some() {
            self.revision += 1;
        }
        self.placed.clear();
        self.shift = [0; 3];
        self.turned = None;
        self.resend = false;
        self.occupied.clear();
        self.occupied_cells.clear();
        self.scan = None;
        self.rescanned = None;
        self.wanted.clear();
        self.miniature_groups.clear();
        self.miniature_over_limit = false;
        self.view = None;
        self.changes.clear();
        self.dropped = None;
        self.overflow = false;
        self.cut = false;
        self.cut_noticed = false;
        self.awaiting = false;
    }

    // Finds the structures a changed marker or a changed library needs. A marker that names other
    // structures drops the old build at once; a move, a turn or a library change keeps it until the
    // new one is assembled.
    fn refresh_source(&mut self, marker: &Marker, library: &Library, loaded: &HashMap<StructureId, Arc<Structure>>) {
        let source = BuildSource::new(marker);
        let changed = self.source.as_ref() != Some(&source);
        if changed {
            if !self.source.as_ref().is_some_and(|current| current.same_blocks(&source)) {
                self.drop_build();
            }
            self.source = Some(source);
        }
        if !changed && !self.library_dirty {
            return;
        }
        self.library_dirty = false;
        self.reassemble = true;
        match GhostAssembler::needed(marker, library) {
            Ok(ids) => {
                self.needed = ids;
                self.awaiting = true;
                if !self.needed.iter().all(|id| loaded.contains_key(id)) {
                    self.drop_build();
                    self.awaiting = true;
                }
            }
            Err(error) => {
                self.refuse(marker, &error);
                self.needed.clear();
                self.drop_build();
            }
        }
    }

    // Builds the ghost as soon as every structure it needs is loaded and, for a turned build, turned
    // on the worker. Only `GhostAssembler::build` runs here; a move or a turn of the same structures
    // keeps the palette and the remaps, so it reads no cell on this thread. A move that only
    // translates the build keeps it and its meshes, drawn moved, until `MOVE_SETTLE_SECONDS` pass
    // with no other move; until then the matching blocks and the tint are those of the old place.
    fn assemble(
        &mut self,
        marker: &Marker,
        library: &Library,
        loaded: &HashMap<StructureId, Arc<Structure>>,
        time_seconds: f64,
    ) {
        self.take_turns();
        if std::mem::take(&mut self.keys_released) && self.shift != [0; 3] {
            self.settle_at = self.settle_at.max(time_seconds + Self::MOVE_SETTLE_SECONDS);
        }
        if !self.awaiting || self.turning.is_some() || !self.needed.iter().all(|id| loaded.contains_key(id)) {
            return;
        }
        if self.shift != [0; 3] && !self.reassemble && !self.settled(time_seconds) {
            return;
        }
        self.awaiting = false;
        let placed = match GhostAssembler::placed(marker, library, loaded) {
            Ok(placed) => placed,
            Err(error) => {
                self.refuse(marker, &error);
                self.drop_build();
                return;
            }
        };
        if self.build.is_some() && self.same_layout(&placed, marker) {
            self.shift = [0; 3];
            return;
        }
        if let Some(delta) = self.translation(&placed, marker) {
            if delta != self.shift {
                self.shift = delta;
                self.settle_at = time_seconds + Self::MOVE_SETTLE_SECONDS;
            }
            if !self.settled(time_seconds) {
                self.awaiting = true;
                self.reassemble = false;
                return;
            }
        }
        let sources: Vec<Arc<Structure>> = placed.iter().map(|piece| Arc::clone(&piece.structure)).collect();
        let rotation = marker.rotation;
        let ready = rotation == Rotation::R0 || self.turned.as_ref().is_some_and(|done| done.turns(&sources, rotation));
        if !ready && !self.request_turn(&sources, rotation) {
            self.awaiting = true;
            return;
        }
        let kept = self.turned.take().filter(|done| rotation != Rotation::R0 && done.turns(&sources, rotation));
        let turned = kept.as_ref().map(|done| done.turned.as_slice()).unwrap_or(&sources);
        let reuse = self.build.as_ref().filter(|_| self.same_structures(&placed));
        match GhostAssembler::build(&placed, turned, marker.origin, rotation, self.revision + 1, reuse) {
            Ok(build) => {
                let same_palette = self.build.as_ref().is_some_and(|current| current.palette == build.palette);
                if same_palette {
                    self.replace(build, rotation);
                } else {
                    self.install(build, rotation);
                }
                self.placed = placed;
                self.pivot = marker.origin;
                self.turned = kept;
            }
            Err(error) => {
                self.refuse(marker, &error);
                self.drop_build();
            }
        }
    }

    // Starts a turn of `sources`. True when it is done already: a worker with no thread turns here.
    fn request_turn(&mut self, sources: &[Arc<Structure>], rotation: Rotation) -> bool {
        if !self.turner.alive() {
            let turned = sources.iter().map(|structure| Arc::new(GhostAssembler::turn(structure, rotation))).collect();
            self.turned = Some(TurnDone { ticket: 0, rotation, sources: sources.to_vec(), turned });
            return true;
        }
        self.ticket += 1;
        let job = TurnJob { ticket: self.ticket, rotation, sources: sources.to_vec() };
        if self.turner.submit(job).is_ok() {
            self.turning = Some(self.ticket);
        }
        false
    }

    // Keeps a finished turn when the marker still asks for its rotation; the next assembly checks
    // that it turns the right structures.
    fn take_turns(&mut self) {
        if self.turning.is_some() && !self.turner.alive() {
            self.turning = None;
            if !self.turner_lost {
                self.turner_lost = true;
                curia::warn!("ghost turn worker gone, turning on the game thread", { ticket: self.ticket });
            }
            self.awaiting = true;
        }
        while let Some(done) = self.turner.take() {
            if self.turning == Some(done.ticket) {
                self.turning = None;
            }
            if self.source.as_ref().is_some_and(|source| source.rotation == done.rotation) {
                self.turned = Some(done);
            }
        }
    }

    // Stops the turn worker and joins it, giving up a turn that runs. Later turns run on the
    // caller's thread.
    pub fn shutdown(&mut self) {
        self.turner.shutdown();
        self.turning = None;
    }

    // True while a turn runs on the worker.
    pub fn turning(&self) -> bool {
        self.turning.is_some()
    }

    // Heap bytes of the turned copies the service holds.
    pub fn turned_bytes(&self) -> u64 {
        self.turned.as_ref().map_or(0, TurnDone::heap_bytes)
    }

    // No HUD key is down and the quiet time after the last move and the last release has passed.
    fn settled(&self, time_seconds: f64) -> bool {
        !self.keys_held && time_seconds >= self.settle_at
    }

    // The move from the current build to `placed`, when `placed` is the same structures turned the
    // same way, each moved by the same amount as the marker. None when there is no build.
    fn translation(&self, placed: &[PlacedStructure], marker: &Marker) -> Option<[i32; 3]> {
        if self.build.is_none() || marker.rotation != self.rotation || !self.same_structures(placed) {
            return None;
        }
        let delta = [0, 1, 2].map(|axis| i64::from(marker.origin[axis]) - i64::from(self.pivot[axis]));
        let moved = self.placed.iter().zip(placed).all(|(old, new)| {
            (0..3).all(|axis| i64::from(new.origin[axis]) - i64::from(old.origin[axis]) == delta[axis])
        });
        if !moved {
            return None;
        }
        let [x, y, z] = delta.map(i32::try_from);
        Some([x.ok()?, y.ok()?, z.ok()?])
    }

    // The same loaded structures in the same order as the current build.
    fn same_structures(&self, placed: &[PlacedStructure]) -> bool {
        self.placed.len() == placed.len()
            && self.placed.iter().zip(placed).all(|(a, b)| Arc::ptr_eq(&a.structure, &b.structure))
    }

    // Logs a refused build; a palette that is too large is also told to the user.
    fn refuse(&mut self, marker: &Marker, error: &GhostError) {
        curia::warn!("ghost build refused", { marker: marker.id.0, error: error.to_string() });
        if matches!(error, GhostError::PaletteTooLarge { .. }) {
            self.refused_notice = true;
        }
    }

    // Keeps the new build in a new generation and scans it once for the sections that hold a cell.
    fn install(&mut self, build: GhostBuild, rotation: Rotation) {
        self.drop_build();
        self.revision = build.revision;
        self.rotation = rotation;
        self.scan = Some(SectionScan::new(&build));
        self.build = Some(build);
    }

    // Puts a moved or turned build of the same palette in place of the current one, in the same
    // generation: the shim keeps its resolved palette and the old meshes stay drawn until the new
    // ones are built. A turn sends the palette again, so the shim turns the block states.
    fn replace(&mut self, mut build: GhostBuild, rotation: Rotation) {
        let Some(current) = self.build.take() else {
            self.install(build, rotation);
            return;
        };
        build.revision = current.revision;
        self.resend |= rotation != self.rotation;
        self.rotation = rotation;
        self.occupied.clear();
        self.occupied_cells.clear();
        self.rescanned = None;
        self.scan = Some(SectionScan::new(&build));
        self.wanted.clear();
        self.miniature_groups.clear();
        self.miniature_over_limit = false;
        self.changes.clear();
        self.dropped = None;
        self.overflow = false;
        self.scheduler.retire_all(self.shift);
        self.shift = [0; 3];
        self.build = Some(build);
    }

    // The same structures at the same places, turned the same way around the same cell, draw the
    // same as the current build.
    fn same_layout(&self, placed: &[PlacedStructure], marker: &Marker) -> bool {
        self.rotation == marker.rotation
            && (self.rotation == Rotation::R0 || self.pivot == marker.origin)
            && self.placed.len() == placed.len()
            && self.placed.iter().zip(placed).all(|(a, b)| a.same_as(b))
    }

    // Marks the slots that hold a changed block as stale, and the wanted full keys that the box of the
    // dropped changes touches. The mini ghost and the marker do not read world blocks.
    fn apply_changes(&mut self) {
        let Some(build) = self.build.as_ref() else {
            self.changes.clear();
            self.dropped = None;
            self.overflow = false;
            return;
        };
        self.overflow = false;
        if let Some((low, high)) = self.dropped.take() {
            for key in self.wanted.iter().filter(|key| !key.part.is_miniature() && key.part != SectionPart::Marker) {
                if Self::padded_overlaps(*key, low, high) {
                    self.scheduler.invalidate(std::slice::from_ref(key));
                }
            }
        }
        if self.changes.is_empty() {
            return;
        }
        self.keys.clear();
        for position in self.changes.drain(..) {
            Self::keys_holding(build, self.view, position, &mut self.keys);
        }
        self.keys.sort_unstable_by_key(|key| (key.x, key.y, key.z, key.layer, key.part));
        self.keys.dedup();
        self.scheduler.invalidate(&self.keys);
    }

    // True when the padded grid of `key` overlaps the inclusive box from `low` to `high`.
    fn padded_overlaps(key: SectionKey, low: [i32; 3], high: [i32; 3]) -> bool {
        let size = i64::from(SectionKey::SIZE);
        let (origin_y, height) = match key.layer {
            Some(layer) => (i64::from(layer), 1),
            None => (i64::from(key.y) * size + i64::from(key.part.y_offset()), i64::from(key.part.height())),
        };
        let origin = [i64::from(key.x) * size, origin_y, i64::from(key.z) * size];
        let dims = [size, height, size];
        (0..3)
            .all(|axis| origin[axis] - 1 <= i64::from(high[axis]) && origin[axis] + dims[axis] >= i64::from(low[axis]))
    }

    // The keys whose padded grid holds `position`: the section of the cell and every neighbour
    // that reads it as padding. Keys without a slot are ignored by the scheduler.
    fn keys_holding(build: &GhostBuild, view: Option<GhostView>, position: [i32; 3], out: &mut Vec<SectionKey>) {
        let inside =
            (0..3).all(|axis| position[axis] >= build.world_min[axis] && position[axis] <= build.world_max[axis]);
        if !inside {
            return;
        }
        let size = SectionKey::SIZE;
        let section = position.map(|v| v.div_euclid(size));
        let holds = |origin: [i64; 3], dims: [i64; 3]| {
            (0..3).all(|axis| {
                let at = i64::from(position[axis]);
                at >= origin[axis] - 1 && at <= origin[axis] + dims[axis]
            })
        };
        for dx in -1..=1_i32 {
            for dy in -1..=1_i32 {
                for dz in -1..=1_i32 {
                    let (x, y, z) = (section[0] + dx, section[1] + dy, section[2] + dz);
                    for part in [SectionPart::Whole, SectionPart::Lower, SectionPart::Upper] {
                        let origin = [
                            i64::from(x) * i64::from(size),
                            i64::from(y) * i64::from(size) + i64::from(part.y_offset()),
                            i64::from(z) * i64::from(size),
                        ];
                        if holds(origin, [i64::from(size), i64::from(part.height()), i64::from(size)]) {
                            out.push(SectionKey { x, y, z, layer: None, part });
                        }
                    }
                }
            }
        }
        if let Some(GhostView::Layer(layer)) = view
            && layer == position[1]
        {
            let y = layer.div_euclid(size);
            for dx in -1..=1_i32 {
                for dz in -1..=1_i32 {
                    let (x, z) = (section[0] + dx, section[2] + dz);
                    let origin = [i64::from(x) * i64::from(size), i64::from(layer), i64::from(z) * i64::from(size)];
                    if holds(origin, [i64::from(size), 1, i64::from(size)]) {
                        out.push(SectionKey { x, y, z, layer: Some(layer), part: SectionPart::Whole });
                    }
                }
            }
        }
    }
}
