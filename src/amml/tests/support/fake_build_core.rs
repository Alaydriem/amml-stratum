use std::collections::VecDeque;

use alaydriem_stratum::traits::BuildCore;
use stratum_core::structs::build::{
    BuildConditions, BuildKindsRequest, BuildKindsResult, BuildProbeRequest, BuildProbeResult,
};

/// The core side of the build calls, with what to hand out and what it was told.
#[derive(Default)]
pub struct FakeBuildCore {
    pub conditions: Vec<BuildConditions>,
    pub probe: Option<BuildProbeRequest>,
    pub probe_answers: Vec<BuildProbeResult>,
    pub kinds: Option<BuildKindsRequest>,
    pub kinds_answers: Vec<BuildKindsResult>,
    pub commands: VecDeque<Vec<String>>,
    pub send_failed: u32,
    pub probe_takes: u32,
    pub command_takes: u32,
    pub panic_on_conditions: bool,
}

impl BuildCore for FakeBuildCore {
    fn set_conditions(&mut self, conditions: BuildConditions) {
        assert!(!self.panic_on_conditions, "a test panic");
        self.conditions.push(conditions);
    }

    fn take_probe(&mut self) -> Option<BuildProbeRequest> {
        self.probe_takes += 1;
        self.probe.take()
    }

    fn apply_probe(&mut self, result: BuildProbeResult) {
        self.probe_answers.push(result);
    }

    fn take_kinds(&mut self) -> Option<BuildKindsRequest> {
        self.kinds.take()
    }

    fn apply_kinds(&mut self, result: BuildKindsResult) {
        self.kinds_answers.push(result);
    }

    fn take_commands(&mut self, out: &mut Vec<String>) {
        self.command_takes += 1;
        if let Some(batch) = self.commands.pop_front() {
            out.extend(batch);
        }
    }

    fn send_failed(&mut self) {
        self.send_failed += 1;
    }
}
