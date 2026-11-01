use crate::structs::library::StructureId;

// Why a set of pieces cannot be laid out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutIssue {
    NoPieces,
    MissingOrigin(Vec<StructureId>),
    Overlap(StructureId, StructureId),
    GridIncomplete(Vec<StructureId>),
    // An offset or the build size does not fit its integer type.
    TooLarge,
}
