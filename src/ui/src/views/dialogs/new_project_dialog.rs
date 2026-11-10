use amml_i18n::text::Text;
use amml_ui::widget::{TextField, TextFieldKind};
use egui::{Context, Id};
use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::snapshot::LibrarySnapshot;

use super::{DialogForm, DialogOutcome, FormAnswer};
use crate::message::StratumMessage;
use crate::presenter::NameCheck;
use crate::views::StratumId;

// The form that names a new project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewProjectDialog {
    name: String,
}

impl NewProjectDialog {
    pub fn new() -> Self {
        Self { name: String::new() }
    }

    pub fn show(&mut self, ctx: &Context, _snapshot: &LibrarySnapshot, out: &mut Vec<LibraryAction>) -> DialogOutcome {
        let blocked = NameCheck::reason(&self.name);
        let name = &mut self.name;
        let form = DialogForm::new(Text::message(StratumMessage::NewProject), Text::message(StratumMessage::Create));
        let answer = form.blocked(blocked).show(ctx, |ui| {
            let placeholder = Text::message(StratumMessage::FactName);
            let width = ui.available_width();
            TextField::new(Id::unique(StratumId::DialogField), name, TextFieldKind::Plain, placeholder).show(ui, width);
        });
        match answer {
            FormAnswer::Pending => DialogOutcome::Open,
            FormAnswer::Cancel => DialogOutcome::Closed,
            FormAnswer::Confirm => {
                let name = self.name.trim();
                if NameCheck::reason(name).is_some() {
                    return DialogOutcome::Open;
                }
                out.push(LibraryAction::CreateProject(name.to_owned()));
                DialogOutcome::Closed
            }
        }
    }
}

impl Default for NewProjectDialog {
    fn default() -> Self {
        Self::new()
    }
}
