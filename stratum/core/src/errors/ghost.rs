use crate::structs::ghost::GhostPalette;
use crate::structs::layout::LayoutIssue;
use crate::structs::library::StructureId;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GhostError {
    #[error("the marker's project or structure is not in the library, or holds no structures")]
    MissingTarget,
    #[error("the pieces cannot be laid out: {0:?}")]
    Layout(LayoutIssue),
    #[error("structure {0:?} is not loaded")]
    NotLoaded(StructureId),
    #[error("the build has more than {} distinct block states", GhostPalette::MAX_ENTRIES)]
    PaletteTooLarge { entries: usize },
}
