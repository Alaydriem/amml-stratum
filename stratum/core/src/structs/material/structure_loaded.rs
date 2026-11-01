use std::sync::Arc;

use crate::errors::StructureError;
use crate::structs::library::StructureId;
use crate::structs::structure::Structure;

// The answer to a `StructureLoad`.
#[derive(Debug, Clone)]
pub struct StructureLoaded {
    pub id: StructureId,
    pub outcome: Result<Arc<Structure>, StructureError>,
}
