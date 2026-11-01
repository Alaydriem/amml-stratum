use std::path::PathBuf;

use super::MaterialScope;
use crate::structs::layout::GridCell;
use crate::structs::library::{ProjectId, ProjectKind, StructureId};
use crate::structs::marker::{MarkerId, MarkerTarget};

// One change the UI asks the controller to make.
#[derive(Debug, Clone, PartialEq)]
pub enum LibraryAction {
    ImportFiles(Vec<PathBuf>),
    // The project the picked files go to, or `None` for Unsorted.
    RequestFilePicker(Option<ProjectId>),
    CreateProject(String),
    RenameProject(ProjectId, String),
    DeleteProject(ProjectId),
    SetProjectKind(ProjectId, ProjectKind),
    RenameStructure(StructureId, String),
    MoveStructure(StructureId, Option<ProjectId>),
    DeleteStructure(StructureId),
    SetGridCell(StructureId, Option<GridCell>),
    // The marker is placed at the player's feet and becomes the active marker.
    PlaceMarker { name: String, target: MarkerTarget },
    ActivateMarker(MarkerId),
    DeactivateMarker,
    RenameMarker(MarkerId, String),
    AssignMarker(MarkerId, MarkerTarget),
    DeleteMarker(MarkerId),
    LoadMaterials(MaterialScope),
}
