use crate::structs::library::StructureId;

// Where one piece sits in a big build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PiecePlacement {
    pub structure: StructureId,
    // Offset from the build origin. Every axis is at least 0.
    pub offset: [i32; 3],
}
