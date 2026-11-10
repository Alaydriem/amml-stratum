// One palette index per cell, in the Bedrock cell order (Z fastest, then Y, then X).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BlockLayer {
    cells: Vec<u16>,
}

impl BlockLayer {
    // The cell holds no block (-1 in the file).
    pub const NONE: u16 = u16::MAX;

    pub fn new(cells: Vec<u16>) -> Self {
        Self { cells }
    }

    // The palette index in a cell, or None for no block or a cell outside the layer.
    pub fn get(&self, cell: usize) -> Option<u16> {
        self.cells.get(cell).copied().filter(|index| *index != Self::NONE)
    }

    // Every cell as stored, `NONE` included.
    pub fn as_slice(&self) -> &[u16] {
        &self.cells
    }

    pub fn len(&self) -> usize {
        self.cells.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    // Heap memory the layer holds.
    pub fn heap_bytes(&self) -> u64 {
        (self.cells.len() as u64).saturating_mul(std::mem::size_of::<u16>() as u64)
    }
}
