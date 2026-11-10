/// A block change that waits for the next tick: the position, and for layer 0 the new block's type name with the
/// time it was seen.
#[derive(Clone, Debug, PartialEq)]
pub struct QueuedChange {
    pub position: [i32; 3],
    pub seen: Option<(Vec<u8>, f64)>,
}
