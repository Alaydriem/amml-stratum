use std::hash::{Hash, Hasher};

use amml_ui::id::IdScope;

// The egui ids of the Stratum views, used with `Id::unique`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StratumId {
    // The menu panel's area.
    Panel,
    // The scroll area of the projects list in the sidebar, which keeps its offset.
    ProjectsScroll,
    // The scroll area of the markers list in the sidebar, which keeps its offset.
    MarkersScroll,
    // The filter field of the markers list.
    MarkerFilter,
    // The project page's tabs.
    ProjectTabs,
    // The structure page's tabs.
    StructureTabs,
    // The filter field of a materials tab.
    MaterialsFilter,
    // The scroll area of a materials table.
    MaterialsScroll,
    // The scroll area of the structure list on a project page.
    StructureListScroll,
    // The scroll area of the card beside the structure list.
    ProjectCardScroll,
    // The open state of the menu beside a project's title.
    ProjectMenu,
    // The scroll area of the structure page's facts card.
    StructureCardScroll,
    // The modal frame of a dialog.
    Dialog,
    // The name field of a dialog.
    DialogField,
    // The list of choices of a dialog.
    DialogChoice,
    // The grid of a big build.
    GridPlacer,
    // The painter layer of the HUD card.
    HudCard,
}

impl Hash for StratumId {
    fn hash<H: Hasher>(&self, state: &mut H) {
        IdScope::hash_type::<Self, H>(state);
        std::mem::discriminant(self).hash(state);
    }
}
