use super::PiecePlacement;
use crate::structs::structure::StructureSize;

// All pieces of a big build, in input order, and the box that holds them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BigBuildLayout {
    pub pieces: Vec<PiecePlacement>,
    pub size: StructureSize,
}
