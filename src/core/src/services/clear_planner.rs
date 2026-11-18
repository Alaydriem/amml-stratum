use crate::structs::build::ClearPart;

// Splits the box of a clear into parts that one fill command can take.
pub struct ClearPlanner;

impl ClearPlanner {
    // The most blocks one fill command may change.
    pub const FILL_LIMIT: u64 = 32_768;

    // The parts of a clear of the box `min` to `max`, both inclusive. With no kinds, each part is
    // one plain fill. With kinds, each part is one replace per kind. A box with a corner below its
    // other corner has no parts.
    pub fn parts(min: [i32; 3], max: [i32; 3], kinds: Option<&[String]>) -> Vec<ClearPart> {
        let mut parts = Vec::new();
        let mut extent = [0_i64; 3];
        for axis in 0..3 {
            extent[axis] = i64::from(max[axis]) - i64::from(min[axis]) + 1;
            if extent[axis] <= 0 {
                return parts;
            }
        }
        // Take all of y first, then x, then z, as far as the limit allows.
        let limit = Self::FILL_LIMIT as i64;
        let step_y = extent[1].min(limit);
        let step_x = extent[0].min(limit / step_y);
        let step_z = extent[2].min(limit / (step_y * step_x));
        let steps = [step_x, step_y, step_z];

        let mut x = 0;
        while x < extent[0] {
            let mut y = 0;
            while y < extent[1] {
                let mut z = 0;
                while z < extent[2] {
                    let low = [x, y, z];
                    let mut part_min = [0_i32; 3];
                    let mut part_max = [0_i32; 3];
                    for axis in 0..3 {
                        let top = (low[axis] + steps[axis]).min(extent[axis]) - 1;
                        part_min[axis] = Self::offset(min[axis], low[axis]);
                        part_max[axis] = Self::offset(min[axis], top);
                    }
                    Self::push(&mut parts, part_min, part_max, kinds);
                    z += step_z;
                }
                y += step_y;
            }
            x += step_x;
        }
        parts
    }

    fn push(parts: &mut Vec<ClearPart>, min: [i32; 3], max: [i32; 3], kinds: Option<&[String]>) {
        match kinds {
            None => parts.push(ClearPart { min, max, kind: None }),
            Some(kinds) => {
                parts.extend(kinds.iter().map(|kind| ClearPart { min, max, kind: Some(kind.clone()) }));
            }
        }
    }

    // `base` plus `delta`, a coordinate inside the box and so inside an i32.
    fn offset(base: i32, delta: i64) -> i32 {
        i32::try_from(i64::from(base) + delta).unwrap_or(i32::MAX)
    }
}
