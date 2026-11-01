// What the ray of the last `aim` call found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AimKind {
    // Easy place does not act on this call.
    Off,
    // No ghost spot under the crosshair.
    Clear,
    // A swap or pick still waits for the server. No spot is read.
    Waiting,
    // A ghost spot that is already pending or tried.
    Known,
    // A ghost spot that needs a decision and a `SpotWorld`.
    Spot,
}
