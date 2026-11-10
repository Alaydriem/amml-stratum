use amml_i18n::text::Text;
use amml_ui::icon::Icon;
use amml_ui::state::DisabledReason;
use amml_ui::widget::{ButtonKind, Leading, TextButton};
use egui::Ui;
use stratum_core::structs::library::ProjectId;
use stratum_core::structs::snapshot::LibrarySnapshot;

use crate::message::StratumMessage;
use crate::state::Page;

// The "Import .mcstructure" button. It asks for the file picker and is off while the picker is open or the library
// cannot be saved.
pub struct ImportButton;

impl ImportButton {
    pub fn build(snapshot: &LibrarySnapshot, kind: ButtonKind) -> TextButton<'static> {
        let allowed = !snapshot.picker_pending && !snapshot.library_read_only;
        let reason = if snapshot.library_read_only {
            DisabledReason::message(StratumMessage::ReadOnlyReason)
        } else {
            DisabledReason::message(StratumMessage::PickerOpenReason)
        };
        TextButton::new(kind, Text::message(StratumMessage::ImportMcstructure))
            .leading(Leading::Icon(Icon::Add))
            .enabled(allowed, reason)
    }

    // The project an import from `page` goes to: the open project, or none for every other page.
    pub fn destination(page: Page) -> Option<ProjectId> {
        match page {
            Page::Project(id) => Some(id),
            _ => None,
        }
    }

    // Draws the button across the width of `ui`; returns true when it was clicked.
    pub fn show_wide(ui: &mut Ui, snapshot: &LibrarySnapshot, kind: ButtonKind) -> bool {
        let width = ui.available_width();
        Self::build(snapshot, kind).width(width).show(ui).clicked()
    }
}
