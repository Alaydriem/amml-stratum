use super::OrbitCamera;
use crate::structs::library::StructureId;

// One picture the UI asks for: a structure file, the view and the image size in pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct PreviewRequest {
    // Counts the requests of one view, so a picture can be matched to the request that made it.
    pub serial: u64,
    pub structure: StructureId,
    // The file name inside the structures folder.
    pub file_name: String,
    pub camera: OrbitCamera,
    pub width: u32,
    pub height: u32,
}
