use super::{BlockShape, Occlusion};

// What the preview draws for one palette entry: the shape, a mesh texture per face in `Face` order, and how
// the block hides its neighbours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BlockLook {
    pub shape: BlockShape,
    pub textures: [u16; 6],
    pub occlusion: Occlusion,
}

impl BlockLook {
    pub fn new(shape: BlockShape, textures: [u16; 6], occlusion: Occlusion) -> Self {
        Self { shape, textures, occlusion }
    }

    pub fn hidden() -> Self {
        Self { shape: BlockShape::Hidden, textures: [0; 6], occlusion: Occlusion::None }
    }
}
