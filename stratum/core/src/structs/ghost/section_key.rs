use super::SectionPart;

// One mesh unit of the ghost build. The section coordinates are world coordinates divided by 16,
// rounded down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SectionKey {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    // The world Y of a Layer-view key.
    pub layer: Option<i32>,
    pub part: SectionPart,
}

impl SectionKey {
    // Cells along each side of a section.
    pub const SIZE: i32 = 16;

    // Squared distance in quarter blocks from `feet` to the centre of the part this key covers.
    pub fn distance_sq(&self, feet: [i32; 3]) -> i64 {
        let centre_y = match self.layer {
            Some(layer) => i64::from(layer) * 2 + 1,
            None => i64::from(self.y) * 32 + i64::from(self.part.y_offset()) * 2 + i64::from(self.part.height()),
        };
        let centre_x = i64::from(self.x) * 32 + 16;
        let centre_z = i64::from(self.z) * 32 + 16;
        let dx = centre_x - i64::from(feet[0]) * 2;
        let dy = centre_y - i64::from(feet[1]) * 2;
        let dz = centre_z - i64::from(feet[2]) * 2;
        dx * dx + dy * dy + dz * dz
    }
}
