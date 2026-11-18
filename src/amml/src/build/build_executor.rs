use std::ffi::c_void;

use amml_game::GameEntries;
use stratum_core::structs::build::BuildConditions;

use crate::build::{ConditionsReader, GameBuildWorld, KindsScanner, PaletteProbe};
use crate::services::HandleState;
use crate::traits::{BuildCore, BuildWorld};
use crate::types::BuildCalls;

/// Runs the build between the game and the core, once a tick after the core's tick. It decides nothing: it reads the
/// conditions and gives them to the core, answers the core's palette and kinds requests from the game, and sends the
/// commands the core gives, in the order it gives them. With no calls (the `stratum.build` feature is off) the
/// conditions are all false and nothing else happens.
///
/// Bound: `commands` is emptied every step and holds at most the core's outbox, whose capacity the config sets.
pub struct BuildExecutor {
    calls: Option<BuildCalls>,
    reader: ConditionsReader,
    scanner: KindsScanner,
    commands: Vec<String>,
    // the conditions last written to the log, so a change is logged once
    logged: Option<BuildConditions>,
}

impl BuildExecutor {
    /// An executor for `calls`; None for a build whose entries are missing, which stays off.
    pub fn new(calls: Option<BuildCalls>) -> Self {
        Self::with_scan_rate(calls, KindsScanner::CELLS_PER_TICK)
    }

    /// As [`BuildExecutor::new`], with the cells the kinds scan reads each tick.
    pub fn with_scan_rate(calls: Option<BuildCalls>, cells_per_tick: usize) -> Self {
        Self {
            reader: ConditionsReader::new(calls.is_some()),
            calls,
            scanner: KindsScanner::new(cells_per_tick),
            commands: Vec::new(),
            logged: None,
        }
    }

    // one log line when the conditions change, with the raw game values they come from
    fn log_conditions(conditions: BuildConditions, world: &dyn BuildWorld) {
        if !world.in_world() {
            curia::info!("build conditions: not in a world", {});
            return;
        }
        curia::info!("build conditions", {
            local: conditions.local,
            cheats: conditions.cheats,
            operator: conditions.operator,
            creative: conditions.creative,
            in_realm: world.in_realm(),
            connection_type: format!("{:?}", world.connection_type()),
            game_type: format!("{:?}", world.game_type()),
            permission_level: format!("{:?}", world.permission_level()),
            commands_flag: format!("{:?}", world.commands_flag()),
        });
    }

    /// Whether the build entries were all there.
    pub fn is_on(&self) -> bool {
        self.calls.is_some()
    }

    /// One tick against the running game and the core's state.
    ///
    /// # Safety
    /// `client` must be null or the live `ClientInstance` of the game build `entries` and the calls were resolved
    /// for, live for the call.
    #[allow(unsafe_code)]
    pub unsafe fn tick(&mut self, client: *mut c_void, entries: &GameEntries, state: &mut HandleState) {
        let Some(calls) = self.calls else {
            state.set_conditions(BuildConditions::default());
            return;
        };
        // SAFETY: forwarded to the caller
        let Some(mut world) = (unsafe { GameBuildWorld::new(client, entries, &calls) }) else {
            if self.logged != Some(BuildConditions::default()) {
                self.logged = Some(BuildConditions::default());
                curia::info!("build conditions: no client world for the build calls", {});
            }
            state.set_conditions(BuildConditions::default());
            return;
        };
        self.step(&mut world, state);
    }

    /// One tick: the conditions to the core, the answers to its requests, then its commands to the game.
    pub fn step(&mut self, world: &mut dyn BuildWorld, core: &mut dyn BuildCore) {
        if self.calls.is_none() {
            core.set_conditions(BuildConditions::default());
            return;
        }
        let conditions = self.reader.read(&*world);
        if self.logged != Some(conditions) {
            self.logged = Some(conditions);
            Self::log_conditions(conditions, &*world);
        }
        core.set_conditions(conditions);
        if let Some(request) = core.take_probe() {
            core.apply_probe(PaletteProbe::answer(&*world, &request));
        }
        if let Some(request) = core.take_kinds() {
            self.scanner.begin(&request);
        }
        if let Some(answer) = self.scanner.step(&*world) {
            core.apply_kinds(answer);
        }
        self.send_commands(world, core);
    }

    // sends what the core wants sent; the first command that is not sent stops the rest and tells the core
    fn send_commands(&mut self, world: &mut dyn BuildWorld, core: &mut dyn BuildCore) {
        self.commands.clear();
        core.take_commands(&mut self.commands);
        for command in &self.commands {
            if !world.send(command) {
                core.send_failed();
                break;
            }
        }
        self.commands.clear();
    }
}
