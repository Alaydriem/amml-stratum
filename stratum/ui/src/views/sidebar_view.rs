use amml_i18n::text::{Args, Text, UserText};
use amml_ui::icon::Icon;
use amml_ui::state::DisabledReason;
use amml_ui::tokens::{SpacingToken, spacing};
use amml_ui::widget::{
    Avatar, AvatarSize, ButtonKind, Caption, Chip, ChipKind, ChipTone, IconButton, Leading, ListRow, NavEntry,
    ScrollBar, SectionHeader, TextButton, TextField, TextFieldKind,
};
use egui::{Align, Id, Layout, ScrollArea, Ui};
use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::marker::{MarkerId, MarkerTarget};
use stratum_core::structs::snapshot::LibrarySnapshot;

use super::dialogs::{DeleteDialog, NewProjectDialog};
use super::{ImportButton, MarkerActions, StratumId};
use crate::message::StratumMessage;
use crate::presenter::{MarkerFilter, SidebarRow, SidebarRows};
use crate::state::{Dialog, DialogItem, LibraryUiState, Page};

// The left column: the pages, the markers of this world and the import button. The rows are rebuilt only when the
// snapshot revision changes.
pub struct SidebarView {
    rows: Vec<SidebarRow>,
    rows_revision: Option<u64>,
}

impl SidebarView {
    /// Paper: Stratum, `Stratum · 13 Import flash + markers list`, sidebar: the gap between the filter field and the
    /// first marker row.
    const FILTER_GAP: f32 = 6.0;

    pub fn new() -> Self {
        Self { rows: Vec::new(), rows_revision: None }
    }

    // The rows of `snapshot`, built again only when its revision is not the one last built.
    pub fn rows(&mut self, snapshot: &LibrarySnapshot) -> &[SidebarRow] {
        if self.rows_revision != Some(snapshot.revision) {
            self.rows = SidebarRows::build(snapshot);
            self.rows_revision = Some(snapshot.revision);
        }
        &self.rows
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        snapshot: &LibrarySnapshot,
        state: &mut LibraryUiState,
        out: &mut Vec<LibraryAction>,
    ) {
        self.rows(snapshot);
        let rows = &self.rows;
        let gap = <spacing::M as SpacingToken>::VALUE;
        // the footer is laid out from the bottom up, so the list takes what the footer leaves
        ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
            Caption::new(Text::message(StratumMessage::ImportHint)).show(ui);
            ui.add_space(<spacing::S as SpacingToken>::VALUE);
            if ImportButton::show_wide(ui, snapshot, ButtonKind::Secondary) {
                out.push(LibraryAction::RequestFilePicker(ImportButton::destination(state.page)));
            }
            ui.add_space(gap);
            ui.with_layout(Layout::top_down(Align::Min), |ui| {
                // the projects list takes at most half the list, the markers list the rest
                let half = ui.available_height() / 2.0;
                let projects = ScrollArea::vertical()
                    .id_salt(StratumId::ProjectsScroll)
                    .max_height(half)
                    .auto_shrink([false, true]);
                ScrollBar::show(ui, projects, false, |ui| Self::show_pages(ui, rows, snapshot, state));
                Self::show_markers(ui, rows, snapshot, state, out);
            });
        });
    }

    // "PROJECTS", the page rows and the "New project" row.
    fn show_pages(ui: &mut Ui, rows: &[SidebarRow], snapshot: &LibrarySnapshot, state: &mut LibraryUiState) {
        SectionHeader::new(Text::message(StratumMessage::ProjectsHeading)).show(ui);
        for row in rows {
            let (page, icon, label, count) = match row {
                SidebarRow::All { count } => {
                    (Page::All, Icon::List, Text::message(StratumMessage::AllStructures), *count)
                }
                SidebarRow::Project { id, name, count } => {
                    (Page::Project(*id), Icon::Folder, Text::user(UserText::from_model(name.clone())), *count)
                }
                SidebarRow::Unsorted { count } => {
                    (Page::Unsorted, Icon::Folder, Text::message(StratumMessage::Unsorted), *count)
                }
                SidebarRow::Marker { .. } => continue,
            };
            let entry = NavEntry::new(icon, label).tally(count).selected(state.page == page);
            if entry.show(ui).clicked() {
                state.page = page;
            }
        }
        let new_project = TextButton::new(ButtonKind::Ghost, Text::message(StratumMessage::NewProject))
            .leading(Leading::Icon(Icon::Add))
            .enabled(!snapshot.library_read_only, DisabledReason::message(StratumMessage::ReadOnlyReason));
        if new_project.show(ui).clicked() {
            state.dialog = Some(Dialog::NewProject(NewProjectDialog::new()));
        }
    }

    // "MARKERS IN THIS WORLD", the filter field when the world has many markers, and one row per marker in a scroll
    // area of its own.
    fn show_markers(
        ui: &mut Ui,
        rows: &[SidebarRow],
        snapshot: &LibrarySnapshot,
        state: &mut LibraryUiState,
        out: &mut Vec<LibraryAction>,
    ) {
        SectionHeader::new(Text::message(StratumMessage::MarkersHeading)).divider(false).show(ui);
        let total = rows.iter().filter(|row| matches!(row, SidebarRow::Marker { .. })).count();
        let filtered = total > MarkerFilter::MARKER_FILTER_THRESHOLD;
        if filtered {
            let id = Id::unique(StratumId::MarkerFilter);
            let placeholder = Text::message(StratumMessage::MarkerFilterPlaceholder);
            let width = ui.available_width();
            TextField::new(id, &mut state.marker_filter, TextFieldKind::Search, placeholder).show(ui, width);
            ui.add_space(Self::FILTER_GAP);
        }
        let filter = if filtered { state.marker_filter.clone() } else { String::new() };
        let area = ScrollArea::vertical().id_salt(StratumId::MarkersScroll).auto_shrink([false, false]);
        ScrollBar::show(ui, area, false, |ui| {
            let mut shown = 0;
            for row in rows {
                let SidebarRow::Marker { id, name, target, target_name, origin, active, missing, part } = row else {
                    continue;
                };
                if !MarkerFilter::matches(name, target_name, part.as_ref().map(|(part, ..)| part.as_str()), &filter) {
                    continue;
                }
                shown += 1;
                let fields = ((*id, name.as_str(), *target), target_name.as_str());
                Self::show_marker(ui, fields, part.as_ref(), (*origin, *active, *missing), (snapshot, state), out);
            }
            if total == 0 {
                Caption::new(Text::message(StratumMessage::NoMarkers)).show(ui);
            } else if shown == 0 {
                Caption::new(Text::message(StratumMessage::NoMarkerMatches)).show(ui);
            }
        });
    }

    fn show_marker(
        ui: &mut Ui,
        ((id, name, target), target_name): ((MarkerId, &str, MarkerTarget), &str),
        part: Option<&(String, u32, u32)>,
        (origin, active, missing): ([i32; 3], bool, bool),
        (snapshot, state): (&LibrarySnapshot, &mut LibraryUiState),
        out: &mut Vec<LibraryAction>,
    ) {
        let model = UserText::from_model(name.to_owned());
        let mut chips = Vec::new();
        if active {
            chips.push(Chip::new(ChipKind::Display(ChipTone::Accent), Text::message(StratumMessage::MarkerActive)));
        }
        if missing {
            chips.push(Chip::new(ChipKind::Display(ChipTone::Warning), Text::message(StratumMessage::MarkerMissing)));
        } else {
            let target = Text::user(UserText::from_model(target_name.to_owned()));
            chips.push(Chip::new(ChipKind::Display(ChipTone::Neutral), target));
            if let Some((part, number, count)) = part {
                let args = Args::new()
                    .user("name", UserText::from_model(part.clone()))
                    .uint("number", u64::from(*number))
                    .uint("count", u64::from(*count));
                chips.push(Chip::new(
                    ChipKind::Display(ChipTone::Neutral),
                    Text::format(StratumMessage::MarkerPart, args),
                ));
            }
        }
        let position =
            Args::new().int("x", i64::from(origin[0])).int("y", i64::from(origin[1])).int("z", i64::from(origin[2]));
        let mut on = active;
        let delete = IconButton::new(Icon::Delete)
            .tooltip(Text::message(StratumMessage::DeleteMarkerTooltip))
            .enabled(!snapshot.markers_read_only, DisabledReason::message(StratumMessage::MarkersReadOnlyReason));
        let mut row = ListRow::new(Avatar::new(&model, AvatarSize::Regular), Text::user(model))
            .version(Text::format(StratumMessage::MarkerPosition, position))
            .chips(chips)
            .selected(state.shows_target(target))
            .action(delete);
        // a marker with no target has nothing to switch on
        if !missing {
            row = row.toggle(&mut on);
        }
        let response = row.show(ui);
        if response.toggled {
            out.push(if on { LibraryAction::ActivateMarker(id) } else { LibraryAction::DeactivateMarker });
        }
        if response.action {
            state.dialog = Some(Dialog::Delete(DeleteDialog::new(DialogItem::Marker(id))));
        } else if response.row.clicked() && !missing {
            state.open_materials(target);
        }
        MarkerActions::context_menu(&response.row, snapshot, id, name, state);
    }
}

impl Default for SidebarView {
    fn default() -> Self {
        Self::new()
    }
}
