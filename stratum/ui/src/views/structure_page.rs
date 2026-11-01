use amml_i18n::text::{Args, Text, UserText};
use amml_ui::widget::{ButtonKind, ScrollBar, TabRow, TextButton};
use egui::{Id, ScrollArea, Ui};
use stratum_core::structs::action::{LibraryAction, MaterialScope};
use stratum_core::structs::snapshot::{LibrarySnapshot, StructureRow};

use super::{FactsPanel, MaterialsTab, PageTitle, StratumId, StructurePreview};
use crate::message::StratumMessage;
use crate::presenter::{DigitGroups, MaterialsSubject};
use crate::state::{LibraryUiState, Page, StructureTab};

// The page of one structure: its title and a way back, then the Facts and Materials tabs.
pub struct StructurePage {
    materials: MaterialsTab,
}

impl StructurePage {
    pub fn new() -> Self {
        Self { materials: MaterialsTab::new() }
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        snapshot: &LibrarySnapshot,
        row: &StructureRow,
        state: &mut LibraryUiState,
        out: &mut Vec<LibraryAction>,
        preview: Option<&mut StructurePreview>,
    ) {
        // the id and name of the structure's project; an unsorted structure has none
        let project = row
            .project
            .and_then(|id| snapshot.projects.iter().find(|project| project.id == id))
            .map(|project| (project.id, project.name.as_str()));
        let size = Args::new()
            .uint("x", u64::from(row.size.x))
            .uint("y", u64::from(row.size.y))
            .uint("z", u64::from(row.size.z))
            .user("blocks", UserText::from_model(DigitGroups::of(row.block_count)));
        let subtitle = match project {
            Some((_, name)) => {
                Text::format(StratumMessage::InProject, size.user("project", UserText::from_model(name.to_owned())))
            }
            None => Text::format(StratumMessage::StructureSubtitle, size),
        };
        let title = PageTitle::new(Text::user(UserText::from_model(row.name.clone()))).meta(vec![subtitle]);
        title.show(ui, |ui| {
            let name = match project {
                Some((_, name)) => Args::new().user("name", UserText::from_model(name.to_owned())),
                None => Args::new().message("name", StratumMessage::Unsorted),
            };
            let back = Text::format(StratumMessage::BackToProject, name);
            if TextButton::new(ButtonKind::Secondary, back).show(ui).clicked() {
                state.page = project.map_or(Page::Unsorted, |(id, _)| Page::Project(id));
            }
        });

        ui.add_space(PageTitle::SECTION_GAP);
        let mut selected = match state.structure_tab {
            StructureTab::Facts => 0,
            StructureTab::Materials => 1,
        };
        let tabs = vec![Text::message(StratumMessage::TabPreview), Text::message(StratumMessage::TabMaterials)];
        TabRow::new(Id::unique(StratumId::StructureTabs), &mut selected, tabs).show(ui);
        state.structure_tab = if selected == 0 { StructureTab::Facts } else { StructureTab::Materials };

        ui.add_space(PageTitle::SECTION_GAP);
        match state.structure_tab {
            StructureTab::Facts => {
                let height = ui.available_height();
                let area = ScrollArea::vertical().id_salt(StratumId::StructureCardScroll).auto_shrink([false, false]);
                ScrollBar::show(ui, area, false, |ui| {
                    out.extend(FactsPanel::show(ui, snapshot, row, state, height, preview));
                });
            }
            StructureTab::Materials => {
                if let Some(subject) = MaterialsSubject::of(snapshot, MaterialScope::Structure(row.id)) {
                    self.materials.show(ui, snapshot, state, &subject, out);
                }
            }
        }
    }
}

impl Default for StructurePage {
    fn default() -> Self {
        Self::new()
    }
}
