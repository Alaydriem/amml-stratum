// A cell of the layout grid. Column runs along +X, row along +Z.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct GridCell {
    pub column: u16,
    pub row: u16,
}

impl GridCell {
    pub fn new(column: u16, row: u16) -> Self {
        Self { column, row }
    }
}
