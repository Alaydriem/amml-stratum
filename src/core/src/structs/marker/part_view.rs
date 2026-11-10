use super::ViewMode;
use crate::structs::library::StructureId;

// The layer and view mode a marker had on one structure of a project, kept while another
// structure is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PartView {
    pub structure: StructureId,
    pub layer: u32,
    pub view_mode: ViewMode,
}
