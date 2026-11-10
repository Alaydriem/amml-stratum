#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StructureSize {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

impl StructureSize {
    pub fn new(x: u32, y: u32, z: u32) -> Self {
        Self { x, y, z }
    }

    // Cells in the bounding box. Saturates at u64::MAX instead of overflowing.
    pub fn volume(&self) -> u64 {
        (u64::from(self.x) * u64::from(self.y)).saturating_mul(u64::from(self.z))
    }
}
