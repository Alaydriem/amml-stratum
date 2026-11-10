use crate::structs::structure::StructureSize;

// The box around the quads of a preview mesh, in sixteenths of a block from the structure's origin. An empty box
// has `min` above `max` on every axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PreviewBounds {
    pub min: [i16; 3],
    pub max: [i16; 3],
}

impl PreviewBounds {
    pub fn empty() -> Self {
        Self { min: [i16::MAX; 3], max: [i16::MIN; 3] }
    }

    // The whole box of a structure of `size`.
    pub fn of_size(size: StructureSize) -> Self {
        let sixteenths = |side: u32| i16::try_from(side.saturating_mul(16)).unwrap_or(i16::MAX);
        Self { min: [0; 3], max: [sixteenths(size.x), sixteenths(size.y), sixteenths(size.z)] }
    }

    pub fn is_empty(&self) -> bool {
        (0..3).any(|axis| self.min[axis] > self.max[axis])
    }

    // Grows the box to hold `corner`.
    pub fn include(&mut self, corner: [i16; 3]) {
        for ((min, max), value) in self.min.iter_mut().zip(self.max.iter_mut()).zip(corner) {
            *min = (*min).min(value);
            *max = (*max).max(value);
        }
    }

    // The lowest corner in blocks.
    pub fn low(&self) -> [f32; 3] {
        self.min.map(|axis| f32::from(axis) / 16.0)
    }

    // The highest corner in blocks.
    pub fn high(&self) -> [f32; 3] {
        self.max.map(|axis| f32::from(axis) / 16.0)
    }
}

impl Default for PreviewBounds {
    fn default() -> Self {
        Self::empty()
    }
}
