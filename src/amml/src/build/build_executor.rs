use stratum_core::structs::build::BuildConditions;

use crate::build::{ConditionsReader, KindsScanner, PaletteProbe};
use crate::traits::{BuildCore, BuildWorld, CommandSink};

/// Runs the build between the game and the core, once a tick after the core's tick. It decides nothing: it reads the
/// conditions and gives them to the core, answers the core's palette and kinds requests from the game, and keeps the
/// commands the core gives, in the order it gives them, for `take_commands`. The caller sends them after it has
/// released its locks, because a command raises game events on the same stack. With the feature off the conditions are all false
/// and nothing else happens.
///
/// Bound: `commands` holds at most the core's outbox, whose capacity the config sets; `take_commands` empties it.
pub struct BuildExecutor {
    on: bool,
    reader: ConditionsReader,
    scanner: KindsScanner,
    commands: Vec<String>,
    // the conditions last handed to the log, so a change is logged once
    logged: Option<BuildConditions>,
    // a change of the conditions the caller has not logged yet
    changed: Option<BuildConditions>,
}

impl BuildExecutor {
    /// An executor; `on` is false when an area of the build is not offered, and the build stays off.
    pub fn new(on: bool) -> Self {
        Self::with_scan_rate(on, KindsScanner::CELLS_PER_TICK)
    }

    /// As [`BuildExecutor::new`], with the cells the kinds scan reads each tick.
    pub fn with_scan_rate(on: bool, cells_per_tick: u32) -> Self {
        Self {
            on,
            reader: ConditionsReader::new(on),
            scanner: KindsScanner::new(cells_per_tick),
            commands: Vec::new(),
            logged: None,
            changed: None,
        }
    }

    /// Whether the build areas were all offered.
    pub fn is_on(&self) -> bool {
        self.on
    }

    /// The conditions, once after each change, for the log.
    pub fn take_changed(&mut self) -> Option<BuildConditions> {
        self.changed.take()
    }

    /// One tick: the conditions to the core, the answers to its requests, then its commands into the send list.
    pub fn step(&mut self, world: &dyn BuildWorld, core: &mut dyn BuildCore) {
        if !self.on {
            core.set_conditions(BuildConditions::default());
            return;
        }
        let conditions = self.reader.read(world);
        if self.logged != Some(conditions) {
            self.logged = Some(conditions);
            self.changed = Some(conditions);
        }
        core.set_conditions(conditions);
        if let Some(request) = core.take_probe() {
            core.apply_probe(PaletteProbe::answer(world, &request));
        }
        if let Some(request) = core.take_kinds() {
            self.scanner.begin(&request);
        }
        if let Some(answer) = self.scanner.step(world) {
            core.apply_kinds(answer);
        }
        core.take_commands(&mut self.commands);
    }

    /// The kept commands, oldest first, for the caller to send once its locks are released; empty when none wait.
    /// Hand the list back with `give_back` so its buffer is used again.
    pub fn take_commands(&mut self) -> Vec<String> {
        if self.commands.is_empty() {
            return Vec::new();
        }
        std::mem::take(&mut self.commands)
    }

    /// The list `take_commands` gave, emptied, for the next tick.
    pub fn give_back(&mut self, mut commands: Vec<String>) {
        commands.clear();
        if self.commands.is_empty() {
            self.commands = commands;
        }
    }

    /// Sends `commands` in order; the first command that is not sent stops the rest, which are dropped, and gives
    /// false: the caller tells the core.
    pub fn send(sink: &mut dyn CommandSink, commands: &[String]) -> bool {
        commands.iter().all(|command| sink.send(command))
    }
}
