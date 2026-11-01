use amml_i18n::text::Text;
use amml_ui::state::DisabledReason;
use amml_ui::tokens::{SpacingToken, spacing};
use amml_ui::widget::{ButtonKind, TextButton};
use egui::Ui;
use stratum_core::structs::snapshot::{LibrarySnapshot, StructureRow};

use super::dialogs::{DeleteDialog, MoveDialog, RenameDialog};
use crate::message::StratumMessage;
use crate::presenter::ItemNames;
use crate::state::{Dialog, DialogItem, LibraryUiState};

// The Rename, Move to project and Delete buttons of a project or structure page.
pub struct ItemActions;

impl ItemActions {
    // `moving` is the structure the Move button is for; a project has none, so it gets no Move button.
    pub fn show(
        ui: &mut Ui,
        snapshot: &LibrarySnapshot,
        item: DialogItem,
        moving: Option<&StructureRow>,
        state: &mut LibraryUiState,
    ) {
        let enabled = !snapshot.library_read_only;
        let reason = || DisabledReason::message(StratumMessage::ReadOnlyReason);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = <spacing::S as SpacingToken>::VALUE;
            let rename = TextButton::new(ButtonKind::Secondary, Text::message(StratumMessage::Rename))
                .enabled(enabled, reason());
            if rename.show(ui).clicked() {
                Self::open_rename(snapshot, item, state);
            }
            if let Some(row) = moving {
                let move_to = TextButton::new(ButtonKind::Secondary, Text::message(StratumMessage::MoveToProject))
                    .enabled(enabled, reason());
                if move_to.show(ui).clicked() {
                    state.dialog = Some(Dialog::Move(MoveDialog::new(row)));
                }
            }
            let delete =
                TextButton::new(ButtonKind::Danger, Text::message(StratumMessage::Delete)).enabled(enabled, reason());
            if delete.show(ui).clicked() {
                Self::open_delete(item, state);
            }
        });
    }

    // Opens the dialog that renames `item`.
    pub(crate) fn open_rename(snapshot: &LibrarySnapshot, item: DialogItem, state: &mut LibraryUiState) {
        let name = ItemNames::item(snapshot, item).unwrap_or_default().to_owned();
        state.dialog = Some(Dialog::Rename(RenameDialog::new(item, name)));
    }

    // Opens the dialog that deletes `item`.
    pub(crate) fn open_delete(item: DialogItem, state: &mut LibraryUiState) {
        state.dialog = Some(Dialog::Delete(DeleteDialog::new(item)));
    }
}
