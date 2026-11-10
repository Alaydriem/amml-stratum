// Where a mesh slot is in its life.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SlotState {
    Free,
    // A job was handed out and has no report yet.
    Pending,
    Built,
    // Built, but a rebuild is wanted.
    Stale,
}
