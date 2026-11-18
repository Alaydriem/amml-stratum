use std::sync::Arc;

use crate::services::{BlockClassifier, BlockStateTurner, ClearPlanner, CommandText};
use crate::structs::build::next_step::NextStep;
use crate::structs::build::plan::BuildPlan;
use crate::structs::build::{
    AssembleRequest, AssembleResult, BuildAnchor, BuildAnswer, BuildJob, BuildKindsRequest, BuildKindsResult,
    BuildNotice, BuildOutbox, BuildPass, BuildPhase, BuildProbeRequest, BuildProbeResult, BuildSettings, BuildStep,
    BuildStopReason, ClearPart, CountRequest, CountResult, OrderRequest, OrderResult, PaletteOutcome,
};
use crate::structs::ghost::{GhostBuild, GhostPalette};
use crate::structs::library::StructureId;
use crate::structs::marker::Rotation;
use crate::structs::snapshot::BuildRow;
use crate::structs::structure::Structure;

// The state of one build, from the settings panel to the last block. It does no I/O: the commands it wants sent go
// to the `BuildOutbox` it is given, and time comes in as an argument.
//
// Bound: the order of one layer (`steps`, and `ahead` for the next layer, worked out by the order worker) is at most the largest layer Stratum
// imports; both are emptied when the build ends. The clear holds one box per fill command, not one part per block
// kind: the parts are made one at a time. `unplaceable` holds at most one name per palette entry.
pub struct BuildSession {
    kinds_capacity: usize,
    structure: StructureId,
    name: String,
    settings: BuildSettings,
    phase: BuildPhase,
    notice: Option<BuildNotice>,
    anchor: Option<BuildAnchor>,
    plan: Option<BuildPlan>,
    // The palette request that no one has taken yet.
    probe: Option<BuildProbeRequest>,
    probe_asked: bool,
    // The job for the worker that no one has taken yet: the assembly of the structure, or the count of blocks.
    pending: Option<BuildJob>,
    // The serial of the assembly, and of the count, the worker has not answered.
    assembling: Option<u64>,
    counting: Option<u64>,
    // The structure could not be placed in the world.
    failed: bool,
    // The kinds request that no one has taken yet.
    kinds: Option<BuildKindsRequest>,
    awaiting_kinds: bool,
    unplaceable: Vec<String>,
    total: u64,
    // The boxes of the clear, one per fill command with no kind filter.
    boxes: Vec<ClearPart>,
    // The kinds the clear replaces; None clears every kind.
    clear_kinds: Option<Vec<String>>,
    clear_total: usize,
    clear_done: usize,
    // The time of the last fill; the next one waits one game tick.
    last_fill: Option<f64>,
    pass: BuildPass,
    layers: u32,
    // Layers loaded in this pass; the layer being placed, counted from 1.
    layer: u32,
    // The index of the layer after the loaded one.
    next_layer: u32,
    steps: Vec<BuildStep>,
    cursor: usize,
    ahead: Option<Vec<BuildStep>>,
    // The serial of the order request the worker has not answered, and the last serial given out.
    outstanding: Option<u64>,
    serial: u64,
    placed: u64,
    // Seconds spent placing, not counting pauses.
    elapsed: f64,
    last_time: f64,
    paused_from: BuildPhase,
    rule_sent: bool,
    // The restore went to the outbox at the end; taken once by `take_restore_queued`.
    restore_queued: bool,
    stop_prompt: Option<f64>,
}

impl BuildSession {
    // The most seconds between two Down presses that stop the build.
    pub const STOP_CONFIRM_SECONDS: f64 = 2.0;
    // The game rule the build turns off while it sends commands.
    // The seconds of one game tick: the clear sends at most one fill in each.
    const GAME_TICK_SECONDS: f64 = 0.05;
    const FEEDBACK_RULE: &'static str = "sendcommandfeedback";

    // The command that puts the game rule back.
    pub fn restore_command() -> String {
        CommandText::game_rule(Self::FEEDBACK_RULE, true)
    }

    // A session in the `Probing` phase, waiting for the ghost build of the structure.
    pub fn new(kinds_capacity: usize, structure: StructureId, name: String, settings: BuildSettings) -> Self {
        Self {
            kinds_capacity,
            structure,
            name,
            settings: settings.clamped(),
            phase: BuildPhase::Probing,
            notice: None,
            anchor: None,
            plan: None,
            probe: None,
            probe_asked: false,
            pending: None,
            assembling: None,
            counting: None,
            failed: false,
            kinds: None,
            awaiting_kinds: false,
            unplaceable: Vec::new(),
            total: 0,
            boxes: Vec::new(),
            clear_kinds: None,
            clear_total: 0,
            clear_done: 0,
            last_fill: None,
            pass: BuildPass::Main,
            layers: 0,
            layer: 0,
            next_layer: 0,
            steps: Vec::new(),
            cursor: 0,
            ahead: None,
            outstanding: None,
            serial: 0,
            placed: 0,
            elapsed: 0.0,
            last_time: 0.0,
            paused_from: BuildPhase::Building,
            rule_sent: false,
            restore_queued: false,
            stop_prompt: None,
        }
    }

    pub fn structure(&self) -> StructureId {
        self.structure
    }

    pub fn phase(&self) -> BuildPhase {
        self.phase
    }

    pub fn settings(&self) -> BuildSettings {
        self.settings
    }

    // True when the build ended: done or stopped.
    pub fn is_finished(&self) -> bool {
        matches!(self.phase, BuildPhase::Done | BuildPhase::Stopped(_))
    }

    // True while the marker must not change.
    pub fn locks_marker(&self) -> bool {
        self.phase.is_running()
    }

    // True while the session waits for the structure to take its blocks from.
    pub fn wants_build(&self) -> bool {
        self.phase == BuildPhase::Probing && self.plan.is_none() && !self.awaiting_kinds && self.assembling.is_none()
    }

    // True while the build holds the game rule changed: from the first command of the build to its restore.
    pub fn rule_changed(&self) -> bool {
        self.rule_sent
    }

    // True once after the end queued the command that puts the game rule back.
    pub fn take_restore_queued(&mut self) -> bool {
        std::mem::take(&mut self.restore_queued)
    }

    pub fn anchor(&self) -> Option<&BuildAnchor> {
        self.anchor.as_ref()
    }

    // Takes the blocks, the box and the order of the structure from its build, and asks the game to look the
    // palette up. A build that is not one piece is left alone.
    pub fn attach(&mut self, build: &GhostBuild, anchor: BuildAnchor) {
        if !self.wants_build() {
            return;
        }
        self.anchor = Some(anchor);
        self.failed = !self.attach_build(build);
    }

    // Asks the worker to place the structure in the world at the marker origin and rotation. The build comes
    // back with `apply_answer`; the session stays `Probing` until then.
    pub fn begin_assemble(
        &mut self,
        anchor: BuildAnchor,
        structure: Arc<Structure>,
        origin: [i32; 3],
        rotation: Rotation,
    ) {
        if !self.wants_build() {
            return;
        }
        self.serial += 1;
        self.assembling = Some(self.serial);
        self.pending = Some(BuildJob::Assemble(AssembleRequest {
            serial: self.serial,
            id: anchor.structure,
            structure,
            origin,
            rotation,
        }));
        self.anchor = Some(anchor);
    }

    // True when the structure could not be placed in the world: the build cannot go on.
    pub fn failed(&self) -> bool {
        self.failed
    }

    // Serials of the jobs this session names start after `from`, so an answer for another session is dropped.
    pub fn start_serials(&mut self, from: u64) {
        self.serial = from;
    }

    fn attach_build(&mut self, build: &GhostBuild) -> bool {
        let [piece] = build.pieces.as_slice() else {
            return false;
        };
        let rotation = build.rotation;
        let blocks: Vec<_> =
            build.palette.entries.iter().map(|block| BlockStateTurner::turn(block, rotation)).collect();
        let turned = GhostPalette { entries: blocks.clone() };
        let mut outcomes = Vec::with_capacity(blocks.len());
        let mut entries = Vec::with_capacity(blocks.len());
        for index in 0..blocks.len() {
            let id = u16::try_from(index).unwrap_or(GhostPalette::NONE);
            let mut source = Vec::new();
            let mut turn = Vec::new();
            let written = build.palette.entry_nbt(id, &mut source).is_ok() && turned.entry_nbt(id, &mut turn).is_ok();
            if written && !source.is_empty() && !turn.is_empty() {
                outcomes.push(PaletteOutcome::Exact);
                entries.push((source, turn));
            } else {
                outcomes.push(PaletteOutcome::Unknown);
                entries.push((Vec::new(), Vec::new()));
            }
        }
        self.layers = piece.structure.size.y;
        self.plan = Some(BuildPlan {
            piece: Arc::new(piece.clone()),
            min: build.world_min,
            max: build.world_max,
            blocks,
            outcomes,
            skip: Arc::new(Vec::new()),
        });
        self.probe = Some(BuildProbeRequest { entries, quarters: rotation.quarters() });
        true
    }

    // The palette request, once.
    pub fn take_probe(&mut self) -> Option<BuildProbeRequest> {
        let request = self.probe.take();
        self.probe_asked |= request.is_some();
        request
    }

    // The game's answer to the palette request. Entries with no answer count as unknown.
    pub fn apply_probe(&mut self, result: &BuildProbeResult) {
        if !self.probe_asked || self.phase != BuildPhase::Probing {
            return;
        }
        let Some(plan) = self.plan.as_mut() else {
            return;
        };
        self.probe_asked = false;
        for (index, outcome) in plan.outcomes.iter_mut().enumerate() {
            if *outcome != PaletteOutcome::Unknown {
                *outcome = result.outcomes.get(index).copied().unwrap_or(PaletteOutcome::Unknown);
            }
        }
        for index in &result.turn_mismatches {
            let block = plan.blocks.get(usize::from(*index)).map(|block| block.name.clone()).unwrap_or_default();
            curia::warn!("the game turned a block differently from core, core's turn is used", { block: block });
        }
        plan.skip = Arc::new(
            plan.piece
                .remap
                .iter()
                .map(|id| {
                    plan.outcomes.get(usize::from(*id)).is_some_and(|outcome| *outcome == PaletteOutcome::Unknown)
                        || plan.blocks.get(usize::from(*id)).is_some_and(BlockClassifier::is_second_half)
                })
                .collect(),
        );
        let mut names: Vec<String> = Vec::new();
        for id in &plan.piece.remap {
            let unknown =
                plan.outcomes.get(usize::from(*id)).is_some_and(|outcome| *outcome == PaletteOutcome::Unknown);
            let Some(block) = plan.blocks.get(usize::from(*id)).filter(|_| unknown) else {
                continue;
            };
            if !names.contains(&block.name) {
                names.push(block.name.clone());
            }
        }
        self.unplaceable = names;
        self.total = 0;
        self.serial += 1;
        self.counting = Some(self.serial);
        self.pending = Some(BuildJob::Count(CountRequest {
            serial: self.serial,
            piece: Arc::clone(&plan.piece),
            skip: Arc::clone(&plan.skip),
        }));
    }

    // The next job for the worker, once.
    pub fn take_job(&mut self) -> Option<BuildJob> {
        if let Some(job) = self.pending.take() {
            return Some(job);
        }
        self.take_order_request().map(BuildJob::Order)
    }

    // The worker did not take the job: it is given again by the next `take_job`.
    pub fn release_job(&mut self, job: BuildJob) {
        match job {
            BuildJob::Order(_) => self.release_order(),
            other => self.pending = Some(other),
        }
    }

    // The worker's answer. An answer to a job that no longer stands is dropped.
    pub fn apply_answer(&mut self, answer: BuildAnswer) {
        match answer {
            BuildAnswer::Order(result) => self.apply_order(result),
            BuildAnswer::Assemble(result) => self.apply_assembled(result),
            BuildAnswer::Count(result) => self.apply_count(result),
        }
    }

    fn apply_assembled(&mut self, result: AssembleResult) {
        if self.assembling != Some(result.serial) {
            return;
        }
        self.assembling = None;
        match result.build {
            Ok(build) => self.failed = !self.attach_build(&build),
            Err(error) => {
                curia::warn!("the structure could not be placed for the build", { error: error.to_string() });
                self.failed = true;
            }
        }
    }

    fn apply_count(&mut self, result: CountResult) {
        if self.counting != Some(result.serial) {
            return;
        }
        self.counting = None;
        self.total = result.total;
        if self.phase == BuildPhase::Probing && !self.awaiting_kinds {
            self.phase = BuildPhase::Ready;
        }
    }

    // Applies new settings while the panel is open. A running build keeps the settings it started with.
    pub fn set_settings(&mut self, settings: BuildSettings) {
        if matches!(self.phase, BuildPhase::Probing | BuildPhase::Ready) {
            self.settings = settings.clamped();
        }
    }

    // Starts the build. With a fluid kept, the box kinds are asked for first. A structure with nothing to place
    // does not start.
    pub fn start(&mut self, now: f64, outbox: &mut BuildOutbox) {
        if self.phase != BuildPhase::Ready || self.total == 0 {
            return;
        }
        let Some(plan) = self.plan.as_ref() else {
            return;
        };
        self.notice = None;
        if self.settings.clear_water && self.settings.clear_lava {
            self.begin(None, now, outbox);
            return;
        }
        self.kinds = Some(BuildKindsRequest { min: plan.min, max: plan.max });
        self.awaiting_kinds = true;
        self.phase = BuildPhase::Probing;
    }

    // The block kinds request, once.
    pub fn take_kinds(&mut self) -> Option<BuildKindsRequest> {
        if self.awaiting_kinds { self.kinds.take() } else { None }
    }

    // The game's answer to the kinds request.
    pub fn apply_kinds(&mut self, result: BuildKindsResult, now: f64, outbox: &mut BuildOutbox) {
        if !self.awaiting_kinds || self.kinds.is_some() {
            return;
        }
        self.awaiting_kinds = false;
        self.phase = BuildPhase::Ready;
        match result {
            BuildKindsResult::NotLoaded => self.notice = Some(BuildNotice::StandNear),
            BuildKindsResult::TooMany => self.notice = Some(BuildNotice::TooManyKinds),
            BuildKindsResult::Kinds(found) if found.len() > self.kinds_capacity => {
                self.notice = Some(BuildNotice::TooManyKinds);
            }
            BuildKindsResult::Kinds(found) => {
                let settings = self.settings;
                let kept = found.into_iter().filter(|kind| Self::cleared(kind, settings)).collect();
                self.begin(Some(kept), now, outbox);
            }
        }
    }

    pub fn pause(&mut self, now: f64) {
        if matches!(self.phase, BuildPhase::Clearing | BuildPhase::Building | BuildPhase::Fluids) {
            self.paused_from = self.phase;
            self.phase = BuildPhase::Paused;
            self.last_time = now;
        }
    }

    pub fn resume(&mut self, now: f64) {
        if self.phase == BuildPhase::Paused {
            self.phase = self.paused_from;
            self.last_time = now;
        }
    }

    // Ends a running build. The game rule is put back, except after `LeftWorld`, when nothing can be sent and the
    // restore stays owed.
    // Commands that wait to be sent are dropped.
    pub fn stop(&mut self, reason: BuildStopReason, outbox: &mut BuildOutbox) {
        if self.phase.is_running() {
            self.end(BuildPhase::Stopped(reason), outbox);
        }
    }

    // A press of the stop key. The first press asks for a second one within `STOP_CONFIRM_SECONDS`; the second
    // stops the build. True when it stopped.
    pub fn press_stop(&mut self, now: f64, outbox: &mut BuildOutbox) -> bool {
        if !self.phase.is_running() {
            return false;
        }
        let confirmed = self.stop_prompt.is_some_and(|at| now - at <= Self::STOP_CONFIRM_SECONDS);
        if confirmed {
            self.stop(BuildStopReason::Operator, outbox);
        } else {
            self.stop_prompt = Some(now);
        }
        confirmed
    }

    // Advances the build to `now`. True when something the row shows changed.
    pub fn tick(&mut self, now: f64, outbox: &mut BuildOutbox) -> bool {
        let delta = (now - self.last_time).max(0.0);
        self.last_time = now;
        let mut changed = false;
        if let Some(at) = self.stop_prompt
            && now - at > Self::STOP_CONFIRM_SECONDS
        {
            self.stop_prompt = None;
            changed = true;
        }
        match self.phase {
            BuildPhase::Clearing => changed |= self.clear_step(now, outbox),
            BuildPhase::Building | BuildPhase::Fluids => {
                self.elapsed += delta;
                changed |= self.place_due(outbox);
            }
            _ => {}
        }
        changed
    }

    pub fn row(&self) -> BuildRow {
        let rate = u64::from(self.settings.blocks_per_second.max(1));
        BuildRow {
            structure: self.structure,
            structure_name: self.name.clone(),
            phase: self.phase,
            settings: self.settings,
            layer: self.layer,
            layers: self.layers,
            placed: self.placed,
            total: self.total,
            eta_seconds: self.total.saturating_sub(self.placed).div_ceil(rate),
            unplaceable: self.unplaceable.clone(),
            notice: self.notice,
            confirm_stop: self.stop_prompt.is_some(),
        }
    }

    // True for a block kind the clear replaces: not air, and not a fluid the settings keep.
    fn cleared(kind: &str, settings: BuildSettings) -> bool {
        let bare = kind.strip_prefix("minecraft:").unwrap_or(kind);
        match bare {
            "air" => false,
            "water" | "flowing_water" => settings.clear_water,
            "lava" | "flowing_lava" => settings.clear_lava,
            _ => true,
        }
    }

    // Queues the game rule change and starts the clear.
    fn begin(&mut self, kinds: Option<Vec<String>>, now: f64, outbox: &mut BuildOutbox) {
        let Some(plan) = self.plan.as_ref() else {
            return;
        };
        let rule = CommandText::game_rule(Self::FEEDBACK_RULE, false);
        if outbox.push(rule).is_err() {
            return;
        }
        self.rule_sent = true;
        self.boxes = ClearPlanner::parts(plan.min, plan.max, None);
        self.clear_total = match &kinds {
            Some(kinds) => self.boxes.len() * kinds.len(),
            None => self.boxes.len(),
        };
        self.clear_kinds = kinds;
        self.clear_done = 0;
        self.last_fill = None;
        self.pass = BuildPass::Main;
        self.layer = 0;
        self.next_layer = 0;
        self.steps.clear();
        self.cursor = 0;
        self.ahead = None;
        self.outstanding = None;
        self.placed = 0;
        self.elapsed = 0.0;
        self.last_time = now;
        self.stop_prompt = None;
        self.phase = BuildPhase::Clearing;
    }

    // One fill command, then the build when the clear is sent.
    fn clear_step(&mut self, now: f64, outbox: &mut BuildOutbox) -> bool {
        let mut changed = false;
        let waited = self.last_fill.is_none_or(|at| now - at >= Self::GAME_TICK_SECONDS - 1e-9);
        if self.clear_done < self.clear_total && waited && outbox.free() > 1 {
            let part = self.clear_part(self.clear_done);
            if outbox.push(CommandText::fill_air(&part)).is_ok() {
                self.clear_done += 1;
                self.last_fill = Some(now);
                changed = true;
            }
        }
        if self.clear_done >= self.clear_total {
            self.boxes = Vec::new();
            self.clear_kinds = None;
            self.elapsed = 0.0;
            self.phase = BuildPhase::Building;
            changed = true;
        }
        changed
    }

    // The part with the number `index`: each box in turn, with each kind in turn.
    fn clear_part(&self, index: usize) -> ClearPart {
        match &self.clear_kinds {
            None => self.boxes[index].clone(),
            Some(kinds) => {
                let area = &self.boxes[index / kinds.len()];
                ClearPart { min: area.min, max: area.max, kind: Some(kinds[index % kinds.len()].clone()) }
            }
        }
    }

    // Queues the blocks that are due, as many as the pace and the room in the outbox allow. When the next layer is
    // not worked out yet, nothing more is sent, and the time spent waiting does not count against the pace.
    fn place_due(&mut self, outbox: &mut BuildOutbox) -> bool {
        let rate = f64::from(self.settings.blocks_per_second);
        let target = (self.elapsed * rate + 1e-9).floor() as u64;
        let due = target.saturating_sub(self.placed);
        // One slot stays free for the command that puts the game rule back.
        let mut room = due.min(outbox.free().saturating_sub(1) as u64);
        let mut changed = false;
        let mut finished = false;
        while room > 0 {
            match self.next_step() {
                NextStep::Step(step) => {
                    let Some(command) = self.command_of(&step) else {
                        continue;
                    };
                    if outbox.push(command).is_err() {
                        break;
                    }
                    self.placed += 1;
                    room -= 1;
                    changed = true;
                }
                NextStep::Waiting => {
                    self.elapsed = self.elapsed.min(self.placed as f64 / rate);
                    break;
                }
                NextStep::Finished => {
                    finished = true;
                    break;
                }
            }
        }
        if finished || self.placed >= self.total {
            self.end(BuildPhase::Done, outbox);
            return true;
        }
        changed
    }

    // The layer the worker should work out next, once: the layer after the loaded one, when it is not ready and
    // not asked for. Answer with `apply_order`.
    pub fn take_order_request(&mut self) -> Option<OrderRequest> {
        let wanted = self.phase.is_running()
            && self.ahead.is_none()
            && self.outstanding.is_none()
            && self.next_layer < self.layers;
        if !wanted {
            return None;
        }
        let plan = self.plan.as_ref()?;
        self.serial += 1;
        self.outstanding = Some(self.serial);
        Some(OrderRequest {
            serial: self.serial,
            pass: self.pass,
            layer: self.next_layer,
            piece: Arc::clone(&plan.piece),
            skip: Arc::clone(&plan.skip),
        })
    }

    // The worker did not take the request: it is made again by the next `take_order_request`.
    pub fn release_order(&mut self) {
        self.outstanding = None;
    }

    // The worker's answer. An answer to a request that no longer stands is dropped.
    pub fn apply_order(&mut self, result: OrderResult) {
        let current =
            self.outstanding == Some(result.serial) && result.pass == self.pass && result.layer == self.next_layer;
        if current {
            self.outstanding = None;
            self.ahead = Some(result.steps);
        }
    }

    fn command_of(&self, step: &BuildStep) -> Option<String> {
        let plan = self.plan.as_ref()?;
        let id = usize::from(*plan.piece.remap.get(usize::from(step.palette))?);
        let block = plan.blocks.get(id)?;
        let outcome = plan.outcomes.get(id).copied()?;
        Some(CommandText::set_block(step.position, block, outcome))
    }

    // The next block of the build: the rest of the layer, else the next layer, else the next pass (main, hanging,
    // fluid).
    fn next_step(&mut self) -> NextStep {
        loop {
            if let Some(step) = self.steps.get(self.cursor).copied() {
                self.cursor += 1;
                return NextStep::Step(step);
            }
            if self.next_layer < self.layers {
                let Some(steps) = self.ahead.take() else {
                    return NextStep::Waiting;
                };
                self.steps = steps;
                self.cursor = 0;
                self.next_layer += 1;
                self.layer = self.next_layer;
                continue;
            }
            match self.pass {
                BuildPass::Fluid => return NextStep::Finished,
                BuildPass::Main => self.pass = BuildPass::Hanging,
                BuildPass::Hanging => {
                    self.pass = BuildPass::Fluid;
                    self.phase = BuildPhase::Fluids;
                }
            }
            self.layer = 0;
            self.next_layer = 0;
            self.steps.clear();
            self.cursor = 0;
            self.ahead = None;
        }
    }

    // Ends the build in `phase`: puts the game rule back, empties the buffers.
    fn end(&mut self, phase: BuildPhase, outbox: &mut BuildOutbox) {
        self.phase = phase;
        self.steps = Vec::new();
        self.ahead = None;
        self.outstanding = None;
        self.pending = None;
        self.boxes = Vec::new();
        self.clear_kinds = None;
        self.cursor = 0;
        self.stop_prompt = None;
        let stopped = matches!(phase, BuildPhase::Stopped(_));
        let left = phase == BuildPhase::Stopped(BuildStopReason::LeftWorld);
        if stopped {
            outbox.clear();
        }
        // After `LeftWorld` nothing can be sent; the service keeps the world owed the restore.
        if self.rule_sent && !left {
            if outbox.push(Self::restore_command()).is_ok() {
                self.restore_queued = true;
            } else {
                curia::error!("the game rule could not be queued for restoring", {});
            }
        }
        self.rule_sent = false;
    }
}
