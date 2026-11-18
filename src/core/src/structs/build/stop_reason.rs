// Why a build ended before it was done.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildStopReason {
    Operator,
    // The player left the world, so nothing can be sent.
    LeftWorld,
    // The player is still in the world but went to another dimension. The game rule is put back.
    DimensionChanged,
    // A condition that allowed the build became false.
    ConditionLost,
    // The active marker was changed, moved, rotated or deleted.
    MarkerChanged,
    StructureDeleted,
    // The mod could not send a command.
    SendFailed,
}
