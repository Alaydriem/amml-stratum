use amml_i18n::text::{Args, Text, UserText};
use amml_ui::widget::{TextField, TextFieldKind};
use egui::{Context, Id};
use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::snapshot::LibrarySnapshot;

use super::{DialogForm, DialogOutcome, FormAnswer};
use crate::message::StratumMessage;
use crate::presenter::{ItemNames, NameCheck};
use crate::state::DialogItem;
use crate::views::StratumId;

// The form that renames a project, a structure or a marker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameDialog {
    item: DialogItem,
    name: String,
}

impl RenameDialog {
    // `name` is what the field holds when the dialog opens, normally the current name.
    pub fn new(item: DialogItem, name: String) -> Self {
        Self { item, name }
    }

    pub fn show(&mut self, ctx: &Context, snapshot: &LibrarySnapshot, out: &mut Vec<LibraryAction>) -> DialogOutcome {
        let Some(current) = ItemNames::item(snapshot, self.item) else {
            return DialogOutcome::Closed;
        };
        let title = Text::format(
            StratumMessage::RenameTitle,
            Args::new().user("name", UserText::from_model(current.to_owned())),
        );
        let blocked = NameCheck::reason(&self.name);
        let name = &mut self.name;
        let form = DialogForm::new(title, Text::message(StratumMessage::Rename));
        let answer = form.blocked(blocked).show(ctx, |ui| {
            let placeholder = Text::message(StratumMessage::FactName);
            let width = ui.available_width();
            TextField::new(Id::unique(StratumId::DialogField), name, TextFieldKind::Plain, placeholder).show(ui, width);
        });
        match answer {
            FormAnswer::Pending => DialogOutcome::Open,
            FormAnswer::Cancel => DialogOutcome::Closed,
            FormAnswer::Confirm => {
                let name = self.name.trim().to_owned();
                if NameCheck::reason(&name).is_some() {
                    return DialogOutcome::Open;
                }
                out.push(match self.item {
                    DialogItem::Project(id) => LibraryAction::RenameProject(id, name),
                    DialogItem::Structure(id) => LibraryAction::RenameStructure(id, name),
                    DialogItem::Marker(id) => LibraryAction::RenameMarker(id, name),
                });
                DialogOutcome::Closed
            }
        }
    }
}
