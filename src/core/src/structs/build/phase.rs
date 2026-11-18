use super::BuildStopReason;

// Where a build stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildPhase {
    // Waiting for the game to look up the blocks, or for the block kinds of the clear box.
    Probing,
    // The settings panel can start the build.
    Ready,
    Clearing,
    Building,
    // Placing water, lava and the gravity blocks that rest on them.
    Fluids,
    Paused,
    Done,
    Stopped(BuildStopReason),
}

impl BuildPhase {
    // True from the start of the clear until the build ends.
    pub fn is_running(self) -> bool {
        matches!(self, Self::Clearing | Self::Building | Self::Fluids | Self::Paused)
    }
}
