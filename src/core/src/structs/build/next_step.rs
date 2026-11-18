use super::BuildStep;

// What the order of a build holds next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NextStep {
    Step(BuildStep),
    // The next layer is not worked out yet.
    Waiting,
    // Both passes are placed.
    Finished,
}
