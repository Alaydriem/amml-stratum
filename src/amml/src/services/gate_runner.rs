use std::path::Path;

use amml_api::amml_abi::{BlockEntityPass, WorldRendered};
use amml_api::{
    ApiError, BlockEntities, BlockEntityCamera, BlockEntityGhost, BlockEntityPassExt, BlockGrid, BlockHandle, BlockPos,
    Blocks, Face, GhostEntityOptions, Logger, MeshId, MesherOptions, SkullPose, StateValue, Transform, Vec3,
    WorldRenderedExt,
};
use stratum_core::services::BlockStateTurner;
use stratum_core::structs::build::{
    BuildConditions, BuildKindsRequest, BuildKindsResult, BuildProbeRequest, PaletteOutcome,
};
use stratum_core::structs::ghost::GhostPalette;
use stratum_core::structs::marker::Rotation;
use stratum_core::structs::structure::{BlockState, BlockStates, StateValue as CoreStateValue};

use crate::build::{KindsScanner, PaletteProbe};
use crate::traits::{BlockGrid as KindsGrid, PaletteGame};

/// The in-game proof of the loader calls stratum makes, with the `gate` feature, on the game thread: on the first tick
/// with a local player it looks up stone by state, turns a stair, and makes one chest ghost and one skull ghost; the
/// world render builds one stone mesh `HEIGHT` blocks above the feet and draws it each frame; the block entity pass
/// draws the two ghosts beside it. With every build condition true it answers one palette probe, scans one 4 x 4 x 4
/// box and asks for one `/setblock` at a fixed spot beside the player, then watches the block change there. Each step
/// logs one line. A world leave logs the meshes it took back and starts over in the next world.
pub struct GateRunner {
    log: Logger,
    shape: String,
    // the first try of the steps, when the next try is due, and the tries so far
    first_try: Option<f64>,
    next_try: f64,
    tries: u32,
    timed_out: bool,
    read_logged: bool,
    looked_up: bool,
    rotated: bool,
    ghosts_made: bool,
    probed: bool,
    scanned: bool,
    stone: Option<BlockHandle>,
    origin: Option<BlockPos>,
    mesh: Option<MeshId>,
    built: bool,
    frames: u64,
    frames_logged: bool,
    chest: Option<BlockEntityGhost>,
    skull: Option<BlockEntityGhost>,
    chest_logged: bool,
    skull_logged: bool,
    target: Option<BlockPos>,
    placed_logged: bool,
}

impl GateRunner {
    /// Blocks between the player's feet and the gate mesh.
    pub const HEIGHT: i32 = 3;
    /// The drawn frames the frames line waits for.
    pub const FRAMES_TO_LOG: u64 = 60;
    /// The block the gate `/setblock` places, and its place from the feet.
    pub const TARGET_BLOCK: &'static str = "minecraft:emerald_block";
    pub const TARGET_OFFSET: [i32; 3] = [2, 0, 2];
    const STONE: &'static str = "minecraft:stone";
    const STAIRS: &'static str = "minecraft:oak_stairs";
    const STAIRS_STATE: &'static str = "weirdo_direction";
    const CHEST: &'static str = "minecraft:chest";
    const SKULL: &'static str = "minecraft:skeleton_skull";
    // a floor skull turned by a quarter
    const SKULL_POSE: SkullPose = SkullPose { face: Face::Up, turn: 4 };
    // the grid of the gate mesh: one stone with a border of empty cells
    const GRID_SIDE: u32 = 3;
    const GRID_CENTRE: usize = 13;
    // the probe's box sides and the kinds a scan of it may hold
    const SCAN_SIDE: i32 = 4;
    const SCAN_KINDS: u32 = 64;
    /// The seconds between two tries of a step that failed, and how long the gate tries before it logs the failures.
    pub const TRY_EVERY: f64 = 1.0;
    pub const TIMEOUT: f64 = 90.0;

    pub fn new(log: Logger, instance_dir: Option<&Path>) -> Self {
        Self {
            log,
            shape: instance_dir.map_or_else(|| "unknown".to_string(), Self::shape_of),
            first_try: None,
            next_try: 0.0,
            tries: 0,
            timed_out: false,
            read_logged: false,
            looked_up: false,
            rotated: false,
            ghosts_made: false,
            probed: false,
            scanned: false,
            stone: None,
            origin: None,
            mesh: None,
            built: false,
            frames: 0,
            frames_logged: false,
            chest: None,
            skull: None,
            chest_logged: false,
            skull_logged: false,
            target: None,
            placed_logged: false,
        }
    }

    /// The skull draw path of the game build: `26.63` for a 26.63 game, `70.x` for a 70 game, read from the game's
    /// package version in the instance folder.
    pub fn shape_of(instance_dir: &Path) -> String {
        let manifest = std::fs::read_to_string(instance_dir.join("AppxManifest.xml")).unwrap_or_default();
        let version = manifest.split("Version=\"1.26.").nth(1).unwrap_or_default();
        match version.get(..2) {
            Some("63") => "26.63".to_string(),
            Some(minor) if minor >= "70" => "70.x".to_string(),
            Some(minor) => format!("26.{minor}"),
            None => "unknown".to_string(),
        }
    }

    /// The client tick at `now` with a local player whose feet are `feet`. Each step that has not succeeded is tried
    /// again every `TRY_EVERY` seconds: the world can join before its blocks and chunks are ready. After `TIMEOUT`
    /// seconds the steps left log their failure once and stop. `ground` is the loader's read of the block under the
    /// feet (its name and handle), logged once as evidence of the block calls. With every build condition true the
    /// probe and the scan run; once the scan has read the loaded box the command is given once; the caller sends it
    /// with no lock held and tells `command_sent`.
    pub fn tick(
        &mut self,
        now: f64,
        feet: [i32; 3],
        ground: Option<(Option<String>, Option<BlockHandle>)>,
        blocks: &Blocks,
        entities: &BlockEntities,
        build: Option<(&BuildConditions, &dyn PaletteGame, &dyn KindsGrid)>,
    ) -> Option<String> {
        if self.timed_out || now < self.next_try {
            return None;
        }
        let first = *self.first_try.get_or_insert(now);
        self.next_try = now + Self::TRY_EVERY;
        self.tries += 1;
        let last = now - first >= Self::TIMEOUT;
        self.log_ground(feet, ground, blocks);
        self.look_up(feet, blocks, entities, last);
        let ready = build.filter(|(conditions, _, _)| {
            conditions.local && conditions.cheats && conditions.operator && conditions.creative
        });
        let mut command = None;
        if let Some((_, palette, grid)) = ready {
            if !self.probed {
                self.probed = self.probe(palette, last);
            }
            if !self.scanned {
                self.scanned = self.scan(feet, grid, last);
                if self.scanned && self.target.is_none() {
                    let [dx, dy, dz] = Self::TARGET_OFFSET;
                    let target = BlockPos { x: feet[0] + dx, y: feet[1] + dy, z: feet[2] + dz };
                    self.target = Some(target);
                    command = Some(format!("/setblock {} {} {} {}", target.x, target.y, target.z, Self::TARGET_BLOCK));
                }
            }
        }
        if last {
            self.timed_out = true;
        }
        command
    }

    // once: the block under the feet as the loader reads it, its handle's name, and whether `from_state` finds a block
    // by that name; the evidence when a lookup by state fails
    fn log_ground(&mut self, feet: [i32; 3], ground: Option<(Option<String>, Option<BlockHandle>)>, blocks: &Blocks) {
        let Some((Some(name), handle)) = ground else { return };
        if self.read_logged {
            return;
        }
        self.read_logged = true;
        let handle_name = handle.and_then(|handle| blocks.name(handle)).unwrap_or_else(|| "none".to_string());
        let by_name = blocks.from_state(&name, &[]).is_some().to_string();
        let pos = format!("{},{},{}", feet[0], feet[1] - 1, feet[2]);
        let has_handle = handle.is_some().to_string();
        self.log.info(
            "gate block read",
            &[
                ("pos", &pos),
                ("name", &name),
                ("handle", &has_handle),
                ("handle_name", &handle_name),
                ("from_state_of_name", &by_name),
                ("tries", &self.tries.to_string()),
            ],
        );
    }

    /// What the loader said to the gate command.
    pub fn command_sent(&mut self, sent: bool) {
        if sent {
            self.log.info("gate build command sent", &[("block", Self::TARGET_BLOCK)]);
        } else {
            self.log.warn("gate build command not sent", &[]);
            self.target = None;
        }
    }

    /// A block change of the local player's dimension; `name` is the new block's full name.
    pub fn block_changed(&mut self, pos: BlockPos, name: Option<&str>) {
        if self.target != Some(pos) {
            return;
        }
        let at = format!("{},{},{}", pos.x, pos.y, pos.z);
        self.log.info("gate block changed", &[("pos", &at), ("name", name.unwrap_or("none"))]);
        if name == Some(Self::TARGET_BLOCK) && !self.placed_logged {
            self.placed_logged = true;
            self.log.info("gate build block placed", &[("pos", &at)]);
        }
    }

    /// The world render: builds the gate mesh once, then draws it.
    pub fn world_rendered(&mut self, event: &WorldRendered) {
        let (Some(stone), Some(origin)) = (self.stone, self.origin) else { return };
        let Some(canvas) = event.canvas() else { return };
        let corner = BlockPos { x: origin.x - 1, y: origin.y - 1, z: origin.z - 1 };
        if !self.built {
            let Some(mesher) = event.mesher() else { return };
            self.built = true;
            let mut cells = [BlockGrid::EMPTY; 27];
            cells[Self::GRID_CENTRE] = 0;
            let palette = [stone];
            let side = Self::GRID_SIDE;
            let grid = BlockGrid { dims: [side; 3], origin: corner, cells: &cells, palette: &palette };
            let options = MesherOptions { alpha: 0x90, skip_matching_world: false, water_tint: false };
            match mesher.build(&grid, &options) {
                Ok(built) => {
                    self.log.info("gate mesh built", &[("quads", &built.quads.to_string())]);
                    self.mesh = (built.quads > 0).then_some(built.mesh);
                }
                Err(error) => self.log.warn("gate mesh not built", &[("error", &format!("{error:?}"))]),
            }
        }
        let Some(mesh) = self.mesh else { return };
        let transform = Transform { origin: corner, offset: Vec3::default(), scale: 1.0, yaw_degrees: 0.0 };
        match canvas.draw(mesh, transform) {
            Ok(()) => {
                self.frames += 1;
                if self.frames >= Self::FRAMES_TO_LOG && !self.frames_logged {
                    self.frames_logged = true;
                    self.log.info("gate drawn", &[("frames", &self.frames.to_string())]);
                }
            }
            Err(ApiError::Stale) => {
                self.mesh = None;
                self.built = false;
            }
            Err(_) => {}
        }
    }

    /// The block entity pass of the player's camera: draws the chest and the skull ghost beside the gate mesh.
    pub fn block_entity_pass(&mut self, event: &BlockEntityPass) {
        if event.camera() != BlockEntityCamera::Player {
            return;
        }
        let Some(origin) = self.origin else { return };
        if let Some(chest) = self.chest.as_ref()
            && event.draw_block_entity(chest, BlockPos { x: origin.x + 2, ..origin }).is_ok()
            && !self.chest_logged
        {
            self.chest_logged = true;
            self.log.info("gate block entity drawn", &[("kind", "chest")]);
        }
        if let Some(skull) = self.skull.as_ref()
            && event.draw_block_entity(skull, BlockPos { x: origin.x - 2, ..origin }).is_ok()
            && !self.skull_logged
        {
            self.skull_logged = true;
            self.log.info("gate skull drawn", &[("shape", &self.shape)]);
        }
    }

    /// The world was left: every handle, mesh and ghost is stale. `renderer_meshes` is what the ghost renderer held.
    pub fn world_left(&mut self, renderer_meshes: usize) {
        let meshes = renderer_meshes + usize::from(self.mesh.is_some());
        self.log.info("gate freed", &[("meshes", &meshes.to_string()), ("reason", "world_left")]);
        let (log, shape) = (self.log, std::mem::take(&mut self.shape));
        *self = Self { shape, ..Self::new(log, None) };
    }

    // the steps that have not succeeded; `last` logs a failure, the tries before it stay quiet
    fn look_up(&mut self, feet: [i32; 3], blocks: &Blocks, entities: &BlockEntities, last: bool) {
        let tries = self.tries.to_string();
        if !self.looked_up {
            match blocks.from_state(Self::STONE, &[]) {
                Some(found) => {
                    self.looked_up = true;
                    self.stone = Some(found.block);
                    self.origin = Some(BlockPos { x: feet[0], y: feet[1] + Self::HEIGHT, z: feet[2] });
                    let exact = found.exact.to_string();
                    self.log.info(
                        "gate block from_state ok",
                        &[("name", Self::STONE), ("exact", &exact), ("tries", &tries)],
                    );
                }
                None if last => {
                    self.log.warn("gate block from_state failed", &[("name", Self::STONE), ("tries", &tries)]);
                }
                None => {}
            }
        }
        if !self.rotated {
            let stairs = blocks.from_state(Self::STAIRS, &[(Self::STAIRS_STATE, StateValue::Int(0))]);
            match stairs.map(|found| (found.block, blocks.rotated(found.block, 1))) {
                Some((block, Ok(turned))) => {
                    self.rotated = true;
                    self.log.info("gate rotated ok", &[("changed", &(turned != block).to_string()), ("tries", &tries)]);
                }
                Some((_, Err(error))) if last => {
                    self.log.warn("gate rotated failed", &[("error", &format!("{error:?}")), ("tries", &tries)]);
                }
                None if last => self.log.warn("gate rotated failed", &[("error", "no stairs"), ("tries", &tries)]),
                _ => {}
            }
        }
        if !self.ghosts_made && self.looked_up {
            if self.chest.is_none() {
                self.chest = self.ghost(blocks, entities, Self::CHEST, &[], GhostEntityOptions::default(), last);
            }
            if self.skull.is_none() {
                let skull_states = [("facing_direction", StateValue::Int(1))];
                let options = GhostEntityOptions { chest_pair: None, skull: Some(Self::SKULL_POSE) };
                self.skull = self.ghost(blocks, entities, Self::SKULL, &skull_states, options, last);
            }
            self.ghosts_made = self.chest.is_some() && self.skull.is_some();
        }
    }

    fn ghost(
        &self,
        blocks: &Blocks,
        entities: &BlockEntities,
        name: &str,
        states: &[(&str, StateValue<'_>)],
        options: GhostEntityOptions,
        last: bool,
    ) -> Option<BlockEntityGhost> {
        let Some(found) = blocks.from_state(name, states) else {
            if last {
                self.log.warn("gate block entity not made", &[("name", name), ("error", "no block")]);
            }
            return None;
        };
        match entities.create(found.block, None, options) {
            Ok(ghost) => Some(ghost),
            Err(error) => {
                if last {
                    self.log.warn("gate block entity not made", &[("name", name), ("error", &format!("{error:?}"))]);
                }
                None
            }
        }
    }

    // one palette probe of stone and a stair turned by a quarter, as core asks for it; true, logged, once an entry is
    // exact, and logged on the last try
    fn probe(&self, palette: &dyn PaletteGame, last: bool) -> bool {
        let stairs = BlockState::new(
            Self::STAIRS.to_string(),
            BlockStates::new(vec![(Self::STAIRS_STATE.to_string(), CoreStateValue::Int(0))]),
        );
        let stone = BlockState::new(Self::STONE.to_string(), BlockStates::default());
        let turned_stairs = BlockStateTurner::turn(&stairs, Rotation::R90);
        let entries = [(stone.clone(), stone), (stairs, turned_stairs)]
            .into_iter()
            .map(|(unturned, turned)| (Self::nbt(unturned), Self::nbt(turned)))
            .collect();
        let result = PaletteProbe::answer(palette, &BuildProbeRequest { entries, quarters: 1 });
        let count = |wanted| result.outcomes.iter().filter(|outcome| **outcome == wanted).count().to_string();
        let found = result.outcomes.contains(&PaletteOutcome::Exact);
        if !found && !last {
            return false;
        }
        self.log.info(
            "gate build probe",
            &[
                ("exact", &count(PaletteOutcome::Exact)),
                ("name_only", &count(PaletteOutcome::NameOnly)),
                ("unknown", &count(PaletteOutcome::Unknown)),
                ("turn_mismatches", &result.turn_mismatches.len().to_string()),
                ("tries", &self.tries.to_string()),
            ],
        );
        found
    }

    fn nbt(state: BlockState) -> Vec<u8> {
        let mut palette = GhostPalette::new();
        palette.entries.push(state);
        let mut out = Vec::new();
        if palette.entry_nbt(0, &mut out).is_err() {
            out.clear();
        }
        out
    }

    // one kinds scan of the 4 x 4 x 4 box under and around the feet; true, logged, once the box is read, and logged on
    // the last try
    fn scan(&self, feet: [i32; 3], grid: &dyn KindsGrid, last: bool) -> bool {
        let min = [feet[0] - 1, feet[1] - 1, feet[2] - 1];
        let max = min.map(|low| low + Self::SCAN_SIDE - 1);
        let mut scanner = KindsScanner::new(Self::SCAN_KINDS * Self::SCAN_KINDS);
        scanner.begin(&BuildKindsRequest { min, max });
        let answer = scanner.step(grid);
        match answer {
            Some(BuildKindsResult::Kinds(kinds)) => {
                let (count, tries) = (kinds.len().to_string(), self.tries.to_string());
                self.log.info("gate build kinds", &[("count", &count), ("kinds", &kinds.join(",")), ("tries", &tries)]);
                true
            }
            other => {
                if last {
                    let tries = self.tries.to_string();
                    self.log.warn("gate build kinds failed", &[("answer", &format!("{other:?}")), ("tries", &tries)]);
                }
                false
            }
        }
    }
}
