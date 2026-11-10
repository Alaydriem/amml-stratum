use amml_i18n::text::{Args, Text, UserText};
use amml_ui::widget::{Dropdown, DropdownOption};
use egui::{Context, Id};
use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::library::{ProjectId, StructureId};
use stratum_core::structs::snapshot::{LibrarySnapshot, StructureRow};

use super::{DialogForm, DialogOutcome, FormAnswer};
use crate::message::StratumMessage;
use crate::presenter::ItemNames;
use crate::views::StratumId;

// The form that moves a structure to a project, or to Unsorted. The destination is kept by project id, so a change
// of the project list cannot retarget it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveDialog {
    structure: StructureId,
    // None until chosen; Some(None) is Unsorted.
    destination: Option<Option<ProjectId>>,
}

impl MoveDialog {
    pub fn new(row: &StructureRow) -> Self {
        Self { structure: row.id, destination: Some(row.project) }
    }

    pub fn show(&mut self, ctx: &Context, snapshot: &LibrarySnapshot, out: &mut Vec<LibraryAction>) -> DialogOutcome {
        let Some(name) = ItemNames::structure(snapshot, self.structure) else {
            return DialogOutcome::Closed;
        };
        let title =
            Text::format(StratumMessage::MoveTitle, Args::new().user("name", UserText::from_model(name.to_owned())));
        // the list: Unsorted, then the projects
        let destinations: Vec<Option<ProjectId>> =
            std::iter::once(None).chain(snapshot.projects.iter().map(|project| Some(project.id))).collect();
        let mut options = vec![DropdownOption::new(Text::message(StratumMessage::Unsorted))];
        for project in &snapshot.projects {
            options.push(DropdownOption::new(Text::user(UserText::from_model(project.name.clone()))));
        }
        let mut index = self.destination.and_then(|chosen| destinations.iter().position(|entry| *entry == chosen));
        let before = index;
        let blocked = index.is_none().then(|| Text::message(StratumMessage::ChooseOneReason));
        let form = DialogForm::new(title, Text::message(StratumMessage::Move));
        let answer = form.blocked(blocked).show(ctx, |ui| {
            let width = ui.available_width();
            Dropdown::new(Id::unique(StratumId::DialogChoice), &mut index, options)
                .placeholder(Text::message(StratumMessage::ChooseOneReason))
                .show(ui, width);
        });
        if index != before {
            self.destination = index.and_then(|picked| destinations.get(picked)).copied();
        }
        match answer {
            FormAnswer::Pending => DialogOutcome::Open,
            FormAnswer::Cancel => DialogOutcome::Closed,
            FormAnswer::Confirm => {
                let listed = self.destination.filter(|chosen| destinations.contains(chosen));
                let Some(destination) = listed else {
                    return DialogOutcome::Open;
                };
                out.push(LibraryAction::MoveStructure(self.structure, destination));
                DialogOutcome::Closed
            }
        }
    }
}
