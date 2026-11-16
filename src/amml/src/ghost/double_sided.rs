/// Which block shapes the game tessellates as one-sided quads that the ghost must draw from both sides. The shape
/// is `BlockGraphics::mBlockShape`, a `BlockShape` value.
pub struct DoubleSided;

impl DoubleSided {
    /// The crossed-quad plants (short grass, fern, flowers, saplings, cobweb), rows (wheat), vines, seagrass,
    /// weeping and twisting vines, cave vines and the chain.
    const SHAPES: [i32; 8] = [1, 6, 20, 97, 121, 122, 123, 129];

    /// Whether a block of `shape` takes the reversed copy of its quads.
    pub fn shape(shape: i32) -> bool {
        Self::SHAPES.contains(&shape)
    }
}
