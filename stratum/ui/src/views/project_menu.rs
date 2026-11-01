use amml_i18n::text::Text;
use amml_ui::icon::Icon;
use amml_ui::state::DisabledReason;
use amml_ui::widget::{IconButton, Menu, MenuItem, MenuLeading, MenuTone};
use egui::{Id, Ui, Vec2};
use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::marker::MarkerTarget;
use stratum_core::structs::snapshot::{LibrarySnapshot, ProjectRow};

use super::{ItemActions, MarkerActions, StratumId};
use crate::message::StratumMessage;
use crate::state::{DialogItem, LibraryUiState};

// The menu beside a project's title: Rename, Assign to marker, Place marker here and Delete for the project.
pub struct ProjectMenu;

impl ProjectMenu {
    // The chevron button, and the menu it opens. `blocked` is a reason that holds back Assign and Place, such as pieces
    // that are not placed yet. Returns the action of Place marker here when it was clicked.
    pub fn show(
        ui: &mut Ui,
        snapshot: &LibrarySnapshot,
        project: &ProjectRow,
        blocked: Option<Text>,
        state: &mut LibraryUiState,
    ) -> Option<LibraryAction> {
        let id = project.id;
        let target = MarkerTarget::Project(id);
        // the button shows as pressed while the menu is open; the flag is the one the last frame ended with, and a change asks for a new frame
        let key = Id::unique(StratumId::ProjectMenu).with(id.0);
        let was_open = ui.data(|data| data.get_temp::<bool>(key)).unwrap_or(false);
        let button = IconButton::new(Icon::ChevronDown)
            .tooltip(Text::message(StratumMessage::ProjectActions))
            .on(was_open)
            .show(ui);
        let writable = !snapshot.library_read_only;
        let read_only = || DisabledReason::message(StratumMessage::ReadOnlyReason);
        let place_reason = MarkerActions::place_reason(snapshot, target, blocked.clone());
        let assign_reason = MarkerActions::assign_reason(snapshot, blocked);
        let mut placed = None;
        let open = Menu::show_from_button(&button, |ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            let rename = MenuItem::new(Text::message(StratumMessage::Rename))
                .leading(MenuLeading::Icon(Icon::Rename))
                .enabled(writable, read_only());
            if rename.show(ui).clicked() {
                ItemActions::open_rename(snapshot, DialogItem::Project(id), state);
            }
            let assign = Self::gate(MenuItem::new(Text::message(StratumMessage::AssignToMarker)), assign_reason);
            if assign.show(ui).clicked() {
                MarkerActions::open_assign(state, target);
            }
            let place =
                MenuItem::new(Text::message(StratumMessage::PlaceMarkerHere)).leading(MenuLeading::Icon(Icon::Pin));
            let place = Self::gate(place, place_reason);
            if place.show(ui).clicked() {
                placed = MarkerActions::place(snapshot, target);
            }
            Menu::separator(ui);
            let delete = MenuItem::new(Text::message(StratumMessage::Delete))
                .leading(MenuLeading::Icon(Icon::Delete))
                .tone(MenuTone::Danger)
                .enabled(writable, read_only());
            if delete.show(ui).clicked() {
                ItemActions::open_delete(DialogItem::Project(id), state);
            }
        });
        ui.data_mut(|data| data.insert_temp(key, open));
        if open != was_open {
            ui.ctx().request_repaint();
        }
        placed
    }

    // The item, held off with `reason` when there is one.
    fn gate(item: MenuItem, reason: Option<Text>) -> MenuItem {
        match reason {
            Some(reason) => item.enabled(false, DisabledReason::new(reason)),
            None => item,
        }
    }
}
