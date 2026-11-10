// How much of the build is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GhostView {
    Whole,
    // One layer at a world Y.
    Layer(i32),
}
