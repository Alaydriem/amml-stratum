// One box of a clear, small enough for one fill command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClearPart {
    // Lowest corner, inclusive.
    pub min: [i32; 3],
    // Highest corner, inclusive.
    pub max: [i32; 3],
    // The block kind this part replaces, or None to fill every block with air.
    pub kind: Option<String>,
}
