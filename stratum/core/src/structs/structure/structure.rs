use super::{BlockLayer, BlockState, SparseLayer, StructureSize};

#[derive(Debug, Clone, PartialEq)]
pub struct Structure {
    pub size: StructureSize,
    pub world_origin: Option<[i32; 3]>,
    pub palette: Vec<BlockState>,
    // First block layer, one entry per cell.
    pub primary: BlockLayer,
    // Second block layer, usually waterlogged liquids. Only cells that hold a block are kept.
    pub secondary: SparseLayer,
}

impl Structure {
    // Position in a layer: Bedrock stores Z fastest, then Y, then X.
    pub fn index_of(&self, x: u32, y: u32, z: u32) -> usize {
        (x as usize)
            .saturating_mul(self.size.y as usize)
            .saturating_add(y as usize)
            .saturating_mul(self.size.z as usize)
            .saturating_add(z as usize)
    }

    // The block in the primary layer, or None for no block or a position outside the box.
    pub fn block_at(&self, x: u32, y: u32, z: u32) -> Option<&BlockState> {
        if x >= self.size.x || y >= self.size.y || z >= self.size.z {
            return None;
        }
        let palette_index = self.primary.get(self.index_of(x, y, z))?;
        self.palette.get(usize::from(palette_index))
    }

    // Primary-layer cells that hold a block other than air or structure void.
    pub fn block_count(&self) -> u64 {
        let counted: Vec<bool> = self.palette.iter().map(|block| !block.is_empty_block()).collect();
        self.primary
            .as_slice()
            .iter()
            .filter(|index| **index != BlockLayer::NONE && counted.get(usize::from(**index)).copied().unwrap_or(false))
            .count() as u64
    }
}
