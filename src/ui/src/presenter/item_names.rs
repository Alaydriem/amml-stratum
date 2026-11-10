use stratum_core::structs::library::{ProjectId, StructureId};
use stratum_core::structs::marker::{MarkerId, MarkerTarget};
use stratum_core::structs::snapshot::{LibrarySnapshot, StructureRow};

use crate::state::DialogItem;

// Looks up the names the dialogs show.
pub struct ItemNames;

impl ItemNames {
    pub fn project(snapshot: &LibrarySnapshot, id: ProjectId) -> Option<&str> {
        snapshot.projects.iter().find(|project| project.id == id).map(|project| project.name.as_str())
    }

    pub fn structure_row(snapshot: &LibrarySnapshot, id: StructureId) -> Option<&StructureRow> {
        snapshot
            .unsorted
            .iter()
            .chain(snapshot.projects.iter().flat_map(|project| project.structures.iter()))
            .find(|row| row.id == id)
    }

    pub fn structure(snapshot: &LibrarySnapshot, id: StructureId) -> Option<&str> {
        Self::structure_row(snapshot, id).map(|row| row.name.as_str())
    }

    pub fn marker(snapshot: &LibrarySnapshot, id: MarkerId) -> Option<&str> {
        snapshot.markers.iter().find(|marker| marker.id == id).map(|marker| marker.name.as_str())
    }

    pub fn target(snapshot: &LibrarySnapshot, target: MarkerTarget) -> Option<&str> {
        match target {
            MarkerTarget::Project(id) => Self::project(snapshot, id),
            MarkerTarget::Structure(id) => Self::structure(snapshot, id),
        }
    }

    pub fn item(snapshot: &LibrarySnapshot, item: DialogItem) -> Option<&str> {
        match item {
            DialogItem::Project(id) => Self::project(snapshot, id),
            DialogItem::Structure(id) => Self::structure(snapshot, id),
            DialogItem::Marker(id) => Self::marker(snapshot, id),
        }
    }
}
