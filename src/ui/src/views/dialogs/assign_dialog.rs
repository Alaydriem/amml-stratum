use amml_i18n::text::{Args, Text, UserText};
use amml_ui::widget::{Dropdown, DropdownOption};
use egui::{Context, Id};
use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::marker::MarkerTarget;
use stratum_core::structs::snapshot::LibrarySnapshot;

use super::{AssignChoice, AssignSubject, DialogForm, DialogOutcome, FormAnswer};
use crate::message::StratumMessage;
use crate::presenter::ItemNames;
use crate::views::StratumId;

// The form that ties a marker to a structure or project. The list holds the markers when a structure or project
// is the subject, and the projects and structures when a marker is. The choice is kept by id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssignDialog {
    subject: AssignSubject,
    choice: Option<AssignChoice>,
}

impl AssignDialog {
    pub fn new(subject: AssignSubject) -> Self {
        Self { subject, choice: None }
    }

    pub fn show(&mut self, ctx: &Context, snapshot: &LibrarySnapshot, out: &mut Vec<LibraryAction>) -> DialogOutcome {
        let subject_name = match self.subject {
            AssignSubject::Target(target) => ItemNames::target(snapshot, target),
            AssignSubject::Marker(id) => ItemNames::marker(snapshot, id),
        };
        let Some(subject_name) = subject_name else {
            return DialogOutcome::Closed;
        };
        let title = Text::format(
            StratumMessage::AssignMenuTitle,
            Args::new().user("name", UserText::from_model(subject_name.to_owned())),
        );
        let choices = match self.subject {
            AssignSubject::Target(_) => Self::markers(snapshot),
            AssignSubject::Marker(_) => Self::targets(snapshot),
        };
        let options = choices
            .iter()
            .map(|(_, name)| DropdownOption::new(Text::user(UserText::from_model((*name).to_owned()))))
            .collect();
        let mut index = self.choice.and_then(|choice| choices.iter().position(|(entry, _)| *entry == choice));
        let before = index;
        let blocked = if choices.is_empty() {
            Some(Text::message(StratumMessage::NoMarkerToAssignReason))
        } else if index.is_none() {
            Some(Text::message(StratumMessage::ChooseOneReason))
        } else {
            None
        };
        let form = DialogForm::new(title, Text::message(StratumMessage::AssignConfirm));
        let answer = form.blocked(blocked).show(ctx, |ui| {
            let width = ui.available_width();
            Dropdown::new(Id::unique(StratumId::DialogChoice), &mut index, options)
                .placeholder(Text::message(StratumMessage::ChooseOneReason))
                .show(ui, width);
        });
        if index != before {
            self.choice = index.and_then(|picked| choices.get(picked)).map(|(choice, _)| *choice);
        }
        match answer {
            FormAnswer::Pending => DialogOutcome::Open,
            FormAnswer::Cancel => DialogOutcome::Closed,
            FormAnswer::Confirm => {
                let still_listed = self.choice.filter(|choice| choices.iter().any(|(entry, _)| entry == choice));
                let action = match (self.subject, still_listed) {
                    (AssignSubject::Target(target), Some(AssignChoice::Marker(marker))) => {
                        Some(LibraryAction::AssignMarker(marker, target))
                    }
                    (AssignSubject::Marker(marker), Some(AssignChoice::Target(target))) => {
                        Some(LibraryAction::AssignMarker(marker, target))
                    }
                    _ => None,
                };
                match action {
                    Some(action) => {
                        out.push(action);
                        DialogOutcome::Closed
                    }
                    None => DialogOutcome::Open,
                }
            }
        }
    }

    fn markers(snapshot: &LibrarySnapshot) -> Vec<(AssignChoice, &str)> {
        snapshot.markers.iter().map(|marker| (AssignChoice::Marker(marker.id), marker.name.as_str())).collect()
    }

    // The projects, then the structures.
    fn targets(snapshot: &LibrarySnapshot) -> Vec<(AssignChoice, &str)> {
        let projects = snapshot
            .projects
            .iter()
            .map(|project| (AssignChoice::Target(MarkerTarget::Project(project.id)), project.name.as_str()));
        let structures = snapshot
            .unsorted
            .iter()
            .chain(snapshot.projects.iter().flat_map(|project| project.structures.iter()))
            .map(|row| (AssignChoice::Target(MarkerTarget::Structure(row.id)), row.name.as_str()));
        projects.chain(structures).collect()
    }
}
