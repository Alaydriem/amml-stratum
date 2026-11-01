use amml_i18n::text::{Args, Text, UserText};
use amml_ui::icon::Icon;
use amml_ui::state::DisabledReason;
use amml_ui::tokens::{SpacingToken, spacing};
use amml_ui::widget::{
    Banner, BannerTone, ButtonKind, Caption, IconButton, Leading, Menu, MenuItem, MenuTone, TextButton,
};
use egui::{Align, Layout, Response, Ui, vec2};
use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::marker::{MarkerId, MarkerTarget, TargetState};
use stratum_core::structs::snapshot::LibrarySnapshot;

use super::dialogs::{AssignDialog, AssignSubject, DeleteDialog, RenameDialog};
use crate::message::StratumMessage;
use crate::presenter::ItemNames;
use crate::state::{Dialog, DialogItem, LibraryUiState};

// What the user can do with markers: place one for a structure or project, assign one, and deal with a marker that
// has nothing to show.
pub struct MarkerActions;

impl MarkerActions {
    // The footer row of a page: "Assign to marker…" and "Place marker here" from the left. `blocked` is a reason both
    // buttons wait for, such as pieces that are not placed yet; it also shows after them. Returns the action of Place
    // marker here when it was clicked.
    pub fn show(
        ui: &mut Ui,
        snapshot: &LibrarySnapshot,
        target: MarkerTarget,
        blocked: Option<Text>,
        state: &mut LibraryUiState,
    ) -> Option<LibraryAction> {
        let hint = blocked.clone();
        let place_reason = Self::place_reason(snapshot, target, blocked.clone());
        let assign_reason = Self::assign_reason(snapshot, blocked);
        let size = vec2(ui.available_width(), <spacing::Button as SpacingToken>::VALUE);
        let mut placed = None;
        ui.allocate_ui_with_layout(size, Layout::left_to_right(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = <spacing::M as SpacingToken>::VALUE;
            let mut assign = TextButton::new(ButtonKind::Secondary, Text::message(StratumMessage::AssignToMarker));
            if let Some(reason) = assign_reason {
                assign = assign.enabled(false, DisabledReason::new(reason));
            }
            if assign.show(ui).clicked() {
                Self::open_assign(state, target);
            }
            let mut place = TextButton::new(ButtonKind::Primary, Text::message(StratumMessage::PlaceMarkerHere))
                .leading(Leading::Icon(Icon::Pin));
            if let Some(reason) = place_reason {
                place = place.enabled(false, DisabledReason::new(reason));
            }
            if place.show(ui).clicked() {
                placed = Self::place(snapshot, target);
            }
            if let Some(hint) = hint {
                Caption::new(hint).show(ui);
            }
        });
        placed
    }

    // A warning for a marker whose structure or project is gone, with a way to assign it a new one. Two or more such
    // markers share one warning that counts them; each is assigned from its menu in the sidebar.
    pub fn show_missing(ui: &mut Ui, snapshot: &LibrarySnapshot, state: &mut LibraryUiState) {
        let missing = snapshot.markers.iter().filter(|marker| marker.state == TargetState::Missing).count();
        if missing > 1 {
            let count = u64::try_from(missing).unwrap_or(u64::MAX);
            Banner::new(BannerTone::Warning, Text::plural(StratumMessage::MarkersMissingTitle, count, Args::new()))
                .body(Text::message(StratumMessage::MarkersMissingBody))
                .show(ui);
            ui.add_space(<spacing::M as SpacingToken>::VALUE);
            return;
        }
        for marker in snapshot.markers.iter().filter(|marker| marker.state == TargetState::Missing) {
            let name = Args::new().user("name", UserText::from_model(marker.name.clone()));
            let assign = TextButton::new(ButtonKind::Outline, Text::message(StratumMessage::Assign))
                .enabled(!snapshot.markers_read_only, DisabledReason::message(StratumMessage::MarkersReadOnlyTitle));
            let banner = Banner::new(BannerTone::Warning, Text::format(StratumMessage::NothingToShow, name))
                .body(Text::message(StratumMessage::NothingToShowBodyUnnamed))
                .action(assign);
            let clicked = banner.show(ui).is_some_and(|response| response.clicked());
            if clicked {
                state.dialog = Some(Dialog::Assign(AssignDialog::new(AssignSubject::Marker(marker.id))));
            }
            ui.add_space(<spacing::M as SpacingToken>::VALUE);
        }
    }

    // The menu a secondary click on a marker row opens: Assign, Rename and Delete marker.
    pub fn context_menu(
        response: &Response,
        snapshot: &LibrarySnapshot,
        id: MarkerId,
        name: &str,
        state: &mut LibraryUiState,
    ) {
        let reason = || DisabledReason::message(StratumMessage::MarkersReadOnlyTitle);
        let writable = !snapshot.markers_read_only;
        Menu::show(response, None, |ui| {
            let assign = MenuItem::new(Text::message(StratumMessage::Assign)).enabled(writable, reason());
            if assign.show(ui).clicked() {
                state.dialog = Some(Dialog::Assign(AssignDialog::new(AssignSubject::Marker(id))));
            }
            let rename = MenuItem::new(Text::message(StratumMessage::Rename)).enabled(writable, reason());
            if rename.show(ui).clicked() {
                let dialog = RenameDialog::new(DialogItem::Marker(id), name.to_owned());
                state.dialog = Some(Dialog::Rename(dialog));
            }
            Menu::separator(ui);
            let delete = MenuItem::new(Text::message(StratumMessage::DeleteMarker))
                .tone(MenuTone::Danger)
                .enabled(writable, reason());
            if delete.show(ui).clicked() {
                state.dialog = Some(Dialog::Delete(DeleteDialog::new(DialogItem::Marker(id))));
            }
        });
    }

    // The trailing button of a structure row: places a marker for `target`, off with the reason placing is off.
    pub(crate) fn row_action(snapshot: &LibrarySnapshot, target: MarkerTarget) -> IconButton {
        let button = IconButton::new(Icon::Pin).tooltip(Text::message(StratumMessage::PlaceMarkerHere));
        match Self::place_reason(snapshot, target, None) {
            Some(reason) => button.enabled(false, DisabledReason::new(reason)),
            None => button,
        }
    }

    // The action that places a marker for `target` at the player's feet, named after the target. None when placing
    // is off.
    pub(crate) fn place(snapshot: &LibrarySnapshot, target: MarkerTarget) -> Option<LibraryAction> {
        if Self::place_reason(snapshot, target, None).is_some() {
            return None;
        }
        let name = ItemNames::target(snapshot, target)?.to_owned();
        Some(LibraryAction::PlaceMarker { name, target })
    }

    // Opens the dialog that assigns a marker to `target`.
    pub(crate) fn open_assign(state: &mut LibraryUiState, target: MarkerTarget) {
        state.dialog = Some(Dialog::Assign(AssignDialog::new(AssignSubject::Target(target))));
    }

    // Why placing a marker for `target` is off, or None when it is on. `blocked` is a reason that holds both buttons.
    pub(crate) fn place_reason(
        snapshot: &LibrarySnapshot,
        target: MarkerTarget,
        blocked: Option<Text>,
    ) -> Option<Text> {
        if blocked.is_some() {
            blocked
        } else if !snapshot.in_world || !snapshot.has_player {
            Some(Text::message(StratumMessage::NotInWorldReason))
        } else if snapshot.markers_read_only {
            Some(Text::message(StratumMessage::MarkersReadOnlyTitle))
        } else if ItemNames::target(snapshot, target).is_none() {
            Some(Text::message(StratumMessage::ReasonNotFound))
        } else {
            None
        }
    }

    // Why assigning a marker is off, or None when it is on.
    pub(crate) fn assign_reason(snapshot: &LibrarySnapshot, blocked: Option<Text>) -> Option<Text> {
        if blocked.is_some() {
            blocked
        } else if snapshot.markers.is_empty() {
            Some(Text::message(StratumMessage::NoMarkerToAssignReason))
        } else if snapshot.markers_read_only {
            Some(Text::message(StratumMessage::MarkersReadOnlyTitle))
        } else {
            None
        }
    }
}
