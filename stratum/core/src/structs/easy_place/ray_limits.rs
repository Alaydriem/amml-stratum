// Where a ghost ray walk ends and which cells it passes through. Distances are along the normalized
// view direction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RayLimits {
    pub max_distance: f64,
    // The first solid cell on the ray. The walk ends before it enters this cell, unless
    // `enter_stop` is set; then the cell can be returned, and the walk ends in it.
    pub stop: Option<[i32; 3]>,
    pub enter_stop: bool,
    // The cells the player body fills, min and max inclusive. They are never a target and the walk
    // passes through them.
    pub body: Option<([i32; 3], [i32; 3])>,
}

impl RayLimits {
    pub fn new(
        max_distance: f64,
        stop: Option<[i32; 3]>,
        enter_stop: bool,
        body: Option<([i32; 3], [i32; 3])>,
    ) -> Self {
        Self { max_distance, stop, enter_stop, body }
    }

    // Only a distance limit.
    pub fn reach(max_distance: f64) -> Self {
        Self::new(max_distance, None, false, None)
    }

    pub fn in_body(&self, cell: [i32; 3]) -> bool {
        self.body.is_some_and(|(min, max)| (0..3).all(|axis| min[axis] <= cell[axis] && cell[axis] <= max[axis]))
    }
}
