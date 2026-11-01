use amml_i18n::text::{Args, Text, UserText};
use amml_ui::icon::Icon;
use amml_ui::widget::{Avatar, AvatarSize, EmptyState, ListRow, ListRowResponse, ScrollBar};
use egui::{Align, Id, ScrollArea, Ui};
use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::marker::MarkerTarget;
use stratum_core::structs::snapshot::{LibrarySnapshot, StructureRow};

use super::{MarkerActions, PageTitle, StratumId};
use crate::message::StratumMessage;
use crate::presenter::DigitGroups;
use crate::state::{LibraryUiState, Page};

// The pages that list structures without a project's actions: All structures, and Unsorted. A click on a row opens
// the structure's page.
pub struct StructureListPage;

impl StructureListPage {
    pub fn show(
        ui: &mut Ui,
        snapshot: &LibrarySnapshot,
        page: Page,
        state: &mut LibraryUiState,
        out: &mut Vec<LibraryAction>,
    ) {
        let rows = Self::rows(snapshot, page);
        let (title, count) = match page {
            Page::Unsorted => (StratumMessage::Unsorted, snapshot.unsorted.len()),
            _ => (StratumMessage::AllStructures, snapshot.all_count),
        };
        let count = u64::try_from(count).unwrap_or(u64::MAX);
        let meta = vec![Text::plural(StratumMessage::StructuresCount, count, Args::new())];
        PageTitle::new(Text::message(title)).meta(meta).show(ui, |_| {});
        ui.add_space(PageTitle::SECTION_GAP);
        if rows.is_empty() {
            let empty = EmptyState::new(
                Icon::Folder,
                Text::message(StratumMessage::UnsortedEmptyTitle),
                Text::message(StratumMessage::UnsortedEmptyBody),
            );
            empty.show(ui);
            return;
        }
        let area =
            ScrollArea::vertical().id_salt(Id::unique(StratumId::StructureListScroll)).auto_shrink([false, false]);
        ScrollBar::show(ui, area, false, |ui| {
            let now = ui.input(|input| input.time);
            for row in rows {
                let strength = state.pick_flash.strength(row.id, now);
                let response = Self::row(ui, snapshot, row, strength);
                if strength > 0.0 {
                    ui.ctx().request_repaint();
                }
                if state.pick_flash.take_scroll(row.id) {
                    response.row.scroll_to_me(Some(Align::Center));
                }
                if response.action {
                    out.extend(MarkerActions::place(snapshot, MarkerTarget::Structure(row.id)));
                } else if response.row.clicked() {
                    state.page = Page::Structure(row.id);
                }
            }
        });
    }

    // The rows of the page, sorted by name ignoring case as core sorts them, then by id.
    fn rows(snapshot: &LibrarySnapshot, page: Page) -> Vec<&StructureRow> {
        match page {
            Page::Unsorted => snapshot.unsorted.iter().collect(),
            _ => {
                let mut rows: Vec<&StructureRow> = snapshot
                    .unsorted
                    .iter()
                    .chain(snapshot.projects.iter().flat_map(|project| project.structures.iter()))
                    .collect();
                rows.sort_by_cached_key(|row| (row.name.to_lowercase(), row.id));
                rows
            }
        }
    }

    fn row(ui: &mut Ui, snapshot: &LibrarySnapshot, row: &StructureRow, flash: f32) -> ListRowResponse {
        let name = UserText::from_model(row.name.clone());
        let size = Args::new()
            .uint("x", u64::from(row.size.x))
            .uint("y", u64::from(row.size.y))
            .uint("z", u64::from(row.size.z))
            .user("blocks", UserText::from_model(DigitGroups::of(row.block_count)));
        ListRow::new(Avatar::new(&name, AvatarSize::Regular), Text::user(name))
            .subtitle(Text::format(StratumMessage::StructureSubtitle, size))
            .flash(flash)
            .action(MarkerActions::row_action(snapshot, MarkerTarget::Structure(row.id)))
            .show(ui)
    }
}
