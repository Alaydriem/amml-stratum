use super::{HudRow, ImportNotice, ImportPick, MarkerRow, ProjectRow, StructureRow};
use crate::structs::action::MaterialScope;
use crate::structs::layout::{BigBuildLayout, LayoutIssue};
use crate::structs::library::ProjectId;
use crate::structs::material::MaterialList;

// An immutable view of the library for the UI. `revision` grows with every change.
#[derive(Debug, Clone, PartialEq)]
pub struct LibrarySnapshot {
    pub revision: u64,
    pub projects: Vec<ProjectRow>,
    pub unsorted: Vec<StructureRow>,
    pub all_count: usize,
    pub markers: Vec<MarkerRow>,
    pub in_world: bool,
    // True when the game has a local player body; placing a marker needs one.
    pub has_player: bool,
    // One entry per big build project, rebuilt from scratch on each library change.
    pub layouts: Vec<(ProjectId, Result<BigBuildLayout, LayoutIssue>)>,
    pub materials: Option<(MaterialScope, MaterialList)>,
    // The scope whose structures are being read; the list arrives on a later tick.
    pub materials_loading: Option<MaterialScope>,
    // The scope whose list could not be made; cleared when a new load starts or a list arrives.
    pub materials_failed: Option<MaterialScope>,
    pub notices: Vec<ImportNotice>,
    pub picker_pending: bool,
    // The most recent file picker import. `None` until a picker answer imports.
    pub latest_pick: Option<ImportPick>,
    // True for the session when the library file could not be read, so changes are not saved.
    pub library_read_only: bool,
    // True for the session once a marker file could not be read; that world's markers are not saved.
    pub markers_read_only: bool,
    // The HUD card of the active marker; None without one.
    pub hud: Option<HudRow>,
}

impl LibrarySnapshot {
    pub fn empty() -> Self {
        Self {
            revision: 0,
            projects: Vec::new(),
            unsorted: Vec::new(),
            all_count: 0,
            markers: Vec::new(),
            in_world: false,
            has_player: false,
            layouts: Vec::new(),
            materials: None,
            materials_loading: None,
            materials_failed: None,
            notices: Vec::new(),
            picker_pending: false,
            latest_pick: None,
            library_read_only: false,
            markers_read_only: false,
            hud: None,
        }
    }
}
