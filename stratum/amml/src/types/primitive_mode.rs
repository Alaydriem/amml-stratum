/// The game's `mce::PrimitiveMode` (one byte), as `Tessellator::begin` takes it.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimitiveMode {
    QuadList = 1,
    TriangleList = 2,
}
