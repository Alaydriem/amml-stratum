use super::PreviewImage;
use crate::errors::PreviewError;
use crate::structs::library::StructureId;

// The answer to one request.
#[derive(Debug, Clone, PartialEq)]
pub struct PreviewFrame {
    pub serial: u64,
    pub structure: StructureId,
    pub outcome: Result<PreviewImage, PreviewError>,
}
