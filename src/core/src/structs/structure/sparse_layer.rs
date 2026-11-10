// The cells of a layer that hold a block, as sorted (cell, palette index) pairs. Used for the
// waterlog layer, where most cells hold nothing, so memory follows the block count, not the volume.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SparseLayer {
    // Strictly increasing cell positions.
    cells: Vec<u32>,
    // The palette index for the cell at the same position in `cells`.
    blocks: Vec<u16>,
}

impl SparseLayer {
    // Pairs `cells[i]` with `blocks[i]`. Unsorted or repeated cells are sorted, and the first
    // pair for a cell is kept; extra items in the longer vector are dropped.
    pub fn new(cells: Vec<u32>, blocks: Vec<u16>) -> Self {
        if cells.len() == blocks.len() && cells.is_sorted_by(|a, b| a < b) {
            return Self { cells, blocks };
        }
        let mut pairs: Vec<(u32, u16)> = cells.into_iter().zip(blocks).collect();
        pairs.sort_by_key(|(cell, _)| *cell);
        pairs.dedup_by_key(|(cell, _)| *cell);
        let (cells, blocks) = pairs.into_iter().unzip();
        Self { cells, blocks }
    }

    // The palette index in a cell, or None when the cell holds no block.
    pub fn get(&self, cell: usize) -> Option<u16> {
        let cell = u32::try_from(cell).ok()?;
        let position = self.cells.binary_search(&cell).ok()?;
        self.blocks.get(position).copied()
    }

    // The (cell, palette index) pairs in cell order.
    pub fn iter(&self) -> impl Iterator<Item = (u32, u16)> + '_ {
        self.cells.iter().copied().zip(self.blocks.iter().copied())
    }

    // Cells that hold a block.
    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    // Heap memory the layer holds.
    pub fn heap_bytes(&self) -> u64 {
        let pair = (std::mem::size_of::<u32>() + std::mem::size_of::<u16>()) as u64;
        (self.cells.len() as u64).saturating_mul(pair)
    }
}
