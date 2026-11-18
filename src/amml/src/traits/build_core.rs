use stratum_core::structs::build::{
    BuildConditions, BuildKindsRequest, BuildKindsResult, BuildProbeRequest, BuildProbeResult,
};

// The core side of the build calls, in the order a tick makes them. The running core answers through its lock; a
// test gives fixed answers
pub trait BuildCore {
    fn set_conditions(&mut self, conditions: BuildConditions);

    fn take_probe(&mut self) -> Option<BuildProbeRequest>;

    fn apply_probe(&mut self, result: BuildProbeResult);

    fn take_kinds(&mut self) -> Option<BuildKindsRequest>;

    fn apply_kinds(&mut self, result: BuildKindsResult);

    // moves the commands the build wants sent to `out`, oldest first
    fn take_commands(&mut self, out: &mut Vec<String>);

    fn send_failed(&mut self);
}
