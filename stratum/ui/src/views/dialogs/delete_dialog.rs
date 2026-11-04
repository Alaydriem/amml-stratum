use amml_i18n::text::{Args, Text, UserText};
use amml_ui::widget::Caption;
use egui::Context;
use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::marker::MarkerTarget;
use stratum_core::structs::snapshot::LibrarySnapshot;

use super::{DialogForm, DialogOutcome, FormAnswer};
use crate::message::StratumMessage;
use crate::presenter::ItemNames;
use crate::state::DialogItem;

// The question before a project, a structure or a marker is deleted. It lists the markers that use the item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteDialog {
    item: DialogItem,
}

impl DeleteDialog {
    pub fn new(item: DialogItem) -> Self {
        Self { item }
    }

    pub fn show(&mut self, ctx: &Context, snapshot: &LibrarySnapshot, out: &mut Vec<LibraryAction>) -> DialogOutcome {
        let Some(name) = ItemNames::item(snapshot, self.item) else {
            return DialogOutcome::Closed;
        };
        let name = UserText::from_model(name.to_owned());
        let lines = self.marker_lines(snapshot);
        let header = (!lines.is_empty()).then(|| {
            let count = u64::try_from(lines.len()).unwrap_or(u64::MAX);
            Text::plural(StratumMessage::MarkersUseIt, count, Args::new())
        });
        let footer = (!lines.is_empty())
            .then(|| Text::format(StratumMessage::WillNoLongerShow, Args::new().user("name", name.clone())));
        let title = Text::format(StratumMessage::DeleteTitle, Args::new().user("name", name));
        let form = DialogForm::new(title, Text::message(StratumMessage::Delete)).danger();
        let answer = form.show(ctx, |ui| {
            if let Some(header) = header {
                Caption::new(header).show(ui);
            }
            for line in lines {
                Caption::new(line).show(ui);
            }
            if let Some(footer) = footer {
                Caption::new(footer).show(ui);
            }
        });
        match answer {
            FormAnswer::Pending => DialogOutcome::Open,
            FormAnswer::Cancel => DialogOutcome::Closed,
            FormAnswer::Confirm => {
                out.push(match self.item {
                    DialogItem::Project(id) => LibraryAction::DeleteProject(id),
                    DialogItem::Structure(id) => LibraryAction::DeleteStructure(id),
                    DialogItem::Marker(id) => LibraryAction::DeleteMarker(id),
                });
                DialogOutcome::Closed
            }
        }
    }

    // One line for each marker that shows the item: "carries" it, or shows the project that holds it.
    fn marker_lines(&self, snapshot: &LibrarySnapshot) -> Vec<Text> {
        let (carried, through) = match self.item {
            DialogItem::Marker(_) => return Vec::new(),
            DialogItem::Project(id) => (MarkerTarget::Project(id), None),
            DialogItem::Structure(id) => {
                let project = ItemNames::structure_row(snapshot, id).and_then(|row| row.project);
                (MarkerTarget::Structure(id), project.map(MarkerTarget::Project))
            }
        };
        let deleted = match self.item {
            DialogItem::Structure(id) => Some(id),
            _ => None,
        };
        let mut lines = Vec::new();
        for marker in &snapshot.markers {
            let cycling_here = deleted.is_some() && marker.current == deleted && Some(marker.target) == through;
            let (message, target) = if marker.target == carried {
                (StratumMessage::MarkerCarries, ItemNames::target(snapshot, marker.target))
            } else if cycling_here {
                let own = deleted.and_then(|id| ItemNames::target(snapshot, MarkerTarget::Structure(id)));
                (StratumMessage::MarkerCarries, own)
            } else if Some(marker.target) == through {
                (StratumMessage::MarkerThrough, ItemNames::target(snapshot, marker.target))
            } else {
                continue;
            };
            let args = Args::new()
                .user("marker", UserText::from_model(marker.name.clone()))
                .user("target", UserText::from_model(target.unwrap_or_default().to_owned()));
            lines.push(Text::format(message, args));
        }
        lines
    }
}
