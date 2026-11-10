use crate::services::PlaceNeeds;
use crate::structs::easy_place::{PlaceNeed, RayLimits};
use crate::structs::ghost::{GhostBuild, GhostView};

// Finds the first ghost cell along a view ray that still needs a block.
pub struct GhostRay;

impl GhostRay {
    // The first cell along `dir` from `eye`, within `limits.max_distance`, whose block has a `Place`
    // need and, in a layer view, lies on that layer. Cells that are not targets, and cells of the
    // player body, are passed through. A ray that starts inside a target returns that cell. The walk
    // ends before the stop cell, or in it when `limits.enter_stop` is set. Distance is along the
    // normalized `dir`. A zero or non-finite `dir` gives None. Allocates nothing.
    pub fn first_spot(
        build: &GhostBuild,
        view: GhostView,
        needs: &PlaceNeeds,
        eye: [f64; 3],
        dir: [f64; 3],
        limits: &RayLimits,
    ) -> Option<([i32; 3], u16)> {
        let max_distance = limits.max_distance;
        let length = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
        if !length.is_finite() || length <= 0.0 || !max_distance.is_finite() || eye.iter().any(|v| !v.is_finite()) {
            return None;
        }
        let dir = dir.map(|v| v / length);

        // The part of the ray inside the build's box, as a distance range.
        let mut enter = 0.0_f64;
        let mut exit = max_distance;
        for axis in 0..3 {
            let low = f64::from(build.world_min[axis]);
            let high = f64::from(build.world_max[axis]) + 1.0;
            if dir[axis] == 0.0 {
                if eye[axis] < low || eye[axis] >= high {
                    return None;
                }
                continue;
            }
            let first = (low - eye[axis]) / dir[axis];
            let second = (high - eye[axis]) / dir[axis];
            enter = enter.max(first.min(second));
            exit = exit.min(first.max(second));
        }
        if enter > exit {
            return None;
        }

        let mut cell = [0_i32; 3];
        let mut step = [0_i32; 3];
        let mut next = [f64::INFINITY; 3];
        let mut span = [f64::INFINITY; 3];
        for axis in 0..3 {
            let at = eye[axis] + dir[axis] * enter;
            cell[axis] = (at.floor() as i32).clamp(build.world_min[axis], build.world_max[axis]);
            if dir[axis] > 0.0 {
                step[axis] = 1;
                next[axis] = (f64::from(cell[axis]) + 1.0 - eye[axis]) / dir[axis];
                span[axis] = 1.0 / dir[axis];
            } else if dir[axis] < 0.0 {
                step[axis] = -1;
                next[axis] = (f64::from(cell[axis]) - eye[axis]) / dir[axis];
                span[axis] = -1.0 / dir[axis];
            }
        }

        let has_stop = limits.stop.is_some();
        let stop = limits.stop.unwrap_or_default();
        let enter_stop = limits.enter_stop;
        loop {
            let at_stop = has_stop && cell[0] == stop[0] && cell[1] == stop[1] && cell[2] == stop[2];
            if at_stop && !enter_stop {
                return None;
            }
            let on_layer = match view {
                GhostView::Whole => true,
                GhostView::Layer(y) => cell[1] == y,
            };
            if on_layer
                && let Some(id) = build.block_at(cell)
                && matches!(needs.need(id), PlaceNeed::Place { .. })
                && !limits.in_body(cell)
            {
                return Some((cell, id));
            }
            if at_stop {
                return None;
            }
            let axis = if next[0] <= next[1] && next[0] <= next[2] {
                0
            } else if next[1] <= next[2] {
                1
            } else {
                2
            };
            if next[axis] > exit {
                return None;
            }
            // A cell step past the box ends the walk.
            let moved = i64::from(cell[axis]) + i64::from(step[axis]);
            if moved < i64::from(build.world_min[axis]) || moved > i64::from(build.world_max[axis]) {
                return None;
            }
            cell[axis] = moved as i32;
            next[axis] += span[axis];
        }
    }
}
