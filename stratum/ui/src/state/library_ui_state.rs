use stratum_core::structs::action::MaterialScope;
use stratum_core::structs::library::StructureId;
use stratum_core::structs::marker::MarkerTarget;
use stratum_core::structs::snapshot::{LibrarySnapshot, StructureRow};

use super::{Dialog, Page, PickFlash, ProjectTab, StructureTab};

// What the menu remembers between frames. The library itself lives in the snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct LibraryUiState {
    pub page: Page,
    pub project_tab: ProjectTab,
    pub structure_tab: StructureTab,
    pub filter: String,
    // The filter of the markers list in the sidebar.
    pub marker_filter: String,
    pub dialog: Option<Dialog>,
    // The snapshot revision the last frame drew.
    pub last_revision: u64,
    // Set when the user closes the menu; the host reads and clears it.
    pub close_requested: bool,
    // The scope whose material list was asked for and has not arrived; cleared when it arrives or the tab is left.
    pub materials_requested: Option<MaterialScope>,
    // The scope the materials tab drew last, so a new scope starts with an empty filter.
    pub materials_shown: Option<MaterialScope>,
    // The scope whose list was being read the last time a frame drew it.
    pub materials_reading: Option<MaterialScope>,
    // The scope whose read ended with no list.
    pub materials_failed: Option<MaterialScope>,
    // Set when the last copy was refused for its size.
    pub copy_too_large: bool,
    // The rows of the latest file picker answer: they flash, and the library opens their page.
    pub pick_flash: PickFlash,
    // The structure the card of a normal project page shows. None, or a structure of another project, shows the first.
    pub selected_structure: Option<StructureId>,
}

impl LibraryUiState {
    pub fn new() -> Self {
        Self {
            page: Page::All,
            project_tab: ProjectTab::Structures,
            structure_tab: StructureTab::Facts,
            filter: String::new(),
            marker_filter: String::new(),
            dialog: None,
            last_revision: 0,
            close_requested: false,
            materials_requested: None,
            materials_shown: None,
            materials_reading: None,
            materials_failed: None,
            copy_too_large: false,
            pick_flash: PickFlash::new(),
            selected_structure: None,
        }
    }

    // The scope of the materials tab the open page shows, if it shows one.
    pub fn materials_scope(&self) -> Option<MaterialScope> {
        match self.page {
            Page::Project(id) if self.project_tab == ProjectTab::Materials => Some(MaterialScope::Project(id)),
            Page::Structure(id) if self.structure_tab == StructureTab::Materials => Some(MaterialScope::Structure(id)),
            _ => None,
        }
    }

    // Opens the materials tab of the page `target` names.
    pub fn open_materials(&mut self, target: MarkerTarget) {
        match target {
            MarkerTarget::Project(id) => {
                self.page = Page::Project(id);
                self.project_tab = ProjectTab::Materials;
            }
            MarkerTarget::Structure(id) => {
                self.page = Page::Structure(id);
                self.structure_tab = StructureTab::Materials;
            }
        }
    }

    // True when the open page is the page of `target`, on any tab.
    pub fn shows_target(&self, target: MarkerTarget) -> bool {
        match (target, self.page) {
            (MarkerTarget::Project(id), Page::Project(open)) => id == open,
            (MarkerTarget::Structure(id), Page::Structure(open)) => id == open,
            _ => false,
        }
    }

    // Moves to All when the page names a project or structure the snapshot no longer has.
    pub fn reconcile(&mut self, snapshot: &LibrarySnapshot) {
        let present = match self.page {
            Page::All | Page::Unsorted => true,
            Page::Project(id) => snapshot.projects.iter().any(|project| project.id == id),
            Page::Structure(id) => {
                snapshot.unsorted.iter().any(|row| row.id == id)
                    || snapshot.projects.iter().any(|project| project.structures.iter().any(|row| row.id == id))
            }
        };
        if !present {
            self.page = Page::All;
        }
        if let Some(id) = self.selected_structure
            && !Self::has_structure(snapshot, id)
        {
            self.selected_structure = None;
        }
    }

    // The structure of `rows` the card shows: the selected one, else the first. Keeps the choice in the state.
    pub fn resolve_selection<'a>(&mut self, rows: &'a [StructureRow]) -> Option<&'a StructureRow> {
        let chosen = rows.iter().find(|row| Some(row.id) == self.selected_structure).or_else(|| rows.first());
        self.selected_structure = chosen.map(|row| row.id);
        chosen
    }

    fn has_structure(snapshot: &LibrarySnapshot, id: StructureId) -> bool {
        snapshot.unsorted.iter().any(|row| row.id == id)
            || snapshot.projects.iter().any(|project| project.structures.iter().any(|row| row.id == id))
    }
}

impl Default for LibraryUiState {
    fn default() -> Self {
        Self::new()
    }
}
