use amml_i18n::text::{Text, UserText};
use amml_ui::draw::{Draw, TextFit, TextStyle, TypeSize};
use amml_ui::font::FontFace;
use amml_ui::icon::Icon;
use amml_ui::state::DisabledReason;
use amml_ui::text::UiText;
use amml_ui::tokens::rule::Elevation;
use amml_ui::tokens::{ColorToken, SpacingToken, color, spacing};
use amml_ui::widget::{
    Banner, BannerTone, ButtonKind, Caption, EmptyState, LoadingKind, LoadingState, ScrollBar, Surface, SurfaceLevel,
    TableAlign, TableColumn, TableHeader, TableRow, TableRowKind, TableRowSize, TextButton, TextField, TextFieldKind,
};
use egui::{Align, Align2, Id, Layout, ScrollArea, Sense, Ui, UiBuilder, vec2};
use stratum_core::structs::action::{LibraryAction, MaterialScope};
use stratum_core::structs::snapshot::LibrarySnapshot;

use super::StratumId;
use crate::message::StratumMessage;
use crate::presenter::{DigitGroups, MaterialClipboard, MaterialRow, MaterialRows, MaterialsSubject, StackText};
use crate::state::LibraryUiState;

// The Materials tab: the heading and totals, the filter, the copy button and the table of items. It asks for the list
// of its scope once and keeps the rows it built until the scope, the snapshot revision or the filter changes.
pub struct MaterialsTab {
    key: Option<(MaterialScope, u64, String)>,
    rows: Vec<MaterialRow>,
}

impl MaterialsTab {
    /// Paper: Stratum, `Stratum · 04 Project materials`, node "Search field": its width.
    const SEARCH_WIDTH: f32 = 220.0;
    /// Paper: Stratum, `Stratum · 04 Project materials`, the COUNT column of "Table header" and every row.
    const COUNT_WIDTH: f32 = 110.0;
    /// Paper: Stratum, `Stratum · 04 Project materials`, the STACKS column of "Table header" and every row.
    const STACKS_WIDTH: f32 = 200.0;
    /// Paper: Mod manager, `Widget / Table header + Table row`: the inset at each side of a row.
    const ROW_INSET: f32 = 18.0;
    const ALIGNS: [TableAlign; 3] = [TableAlign::Start, TableAlign::End, TableAlign::End];
    const HEADING: TextStyle = TextStyle::new(TypeSize::Body, FontFace::Semibold);
    const SUMMARY: TextStyle = TextStyle::new(TypeSize::Caption, FontFace::Regular);
    const CELL: TextStyle = TextStyle::new(TypeSize::Body, FontFace::Regular);
    const CELL_DIM: TextStyle = TextStyle::new(TypeSize::Help, FontFace::Regular);

    pub fn new() -> Self {
        Self { key: None, rows: Vec::new() }
    }

    // The rows the table shows for `scope` and the filter in `state`, built again only when one of them or the snapshot
    // revision changed.
    pub fn rows(&mut self, snapshot: &LibrarySnapshot, state: &LibraryUiState, scope: MaterialScope) -> &[MaterialRow] {
        let key = (scope, snapshot.revision, state.filter.clone());
        if self.key.as_ref() != Some(&key) {
            self.rows = match &snapshot.materials {
                Some((listed, list)) if *listed == scope => MaterialRows::build(list, &state.filter),
                _ => Vec::new(),
            };
            self.key = Some(key);
        }
        &self.rows
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        snapshot: &LibrarySnapshot,
        state: &mut LibraryUiState,
        subject: &MaterialsSubject,
        out: &mut Vec<LibraryAction>,
    ) {
        let scope = subject.scope;
        if state.materials_shown != Some(scope) {
            state.materials_shown = Some(scope);
            state.filter.clear();
            state.copy_too_large = false;
            state.materials_reading = None;
            state.materials_failed = None;
        }
        let list = snapshot.materials.as_ref().filter(|(listed, _)| *listed == scope).map(|(_, list)| list);
        let loading = snapshot.materials_loading == Some(scope);
        if list.is_some() {
            state.materials_requested = None;
            state.materials_reading = None;
            state.materials_failed = None;
        } else if loading {
            state.materials_reading = Some(scope);
            state.materials_failed = None;
        } else if state.materials_reading == Some(scope) || snapshot.materials_failed == Some(scope) {
            // the read ended and left no list
            state.materials_reading = None;
            state.materials_requested = None;
            state.materials_failed = Some(scope);
        } else if state.materials_failed != Some(scope) && state.materials_requested != Some(scope) {
            state.materials_requested = Some(scope);
            out.push(LibraryAction::LoadMaterials(scope));
        }
        if self.key.as_ref().is_some_and(|(_, _, filter)| *filter != state.filter) {
            state.copy_too_large = false;
        }
        let item_count = list.map(|list| list.lines().len());
        self.rows(snapshot, state, scope);

        self.show_toolbar(ui, state, subject, item_count);
        if state.copy_too_large {
            ui.add_space(<spacing::M as SpacingToken>::VALUE);
            Banner::new(BannerTone::Warning, Text::message(StratumMessage::CopyTooLarge))
                .body(Text::message(StratumMessage::CopyTooLargeBody))
                .show(ui);
        }
        ui.add_space(<spacing::L as SpacingToken>::VALUE);
        if state.materials_failed == Some(scope) && list.is_none() {
            let retry = TextButton::new(ButtonKind::Secondary, Text::message(StratumMessage::TryAgain));
            let failed = EmptyState::new(
                Icon::Warning,
                Text::message(StratumMessage::MaterialsFailed),
                Text::message(StratumMessage::MaterialsFailedBody),
            )
            .action(retry);
            if failed.show(ui).is_some_and(|action| action.clicked()) {
                state.materials_failed = None;
                state.materials_requested = Some(scope);
                out.push(LibraryAction::LoadMaterials(scope));
            }
            return;
        }
        if list.is_none() {
            LoadingState::new(LoadingKind::Inline(Text::message(StratumMessage::MaterialsLoading))).show(ui);
            return;
        }
        self.show_table(ui, state);
    }

    fn show_toolbar(&self, ui: &mut Ui, state: &mut LibraryUiState, subject: &MaterialsSubject, items: Option<usize>) {
        let gap = <spacing::M as SpacingToken>::VALUE;
        let height = <spacing::Button as SpacingToken>::VALUE;
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        let mut right = ui.new_child(UiBuilder::new().max_rect(rect).layout(Layout::right_to_left(Align::Center)));
        let copy = TextButton::new(ButtonKind::Primary, subject.copy_label())
            .enabled(!self.rows.is_empty(), DisabledReason::message(StratumMessage::MaterialsEmpty));
        if copy.show(&mut right).clicked() {
            let ctx = ui.ctx().clone();
            let text = MaterialClipboard::render(&self.rows, &subject.name, |text| UiText::resolve(&ctx, text));
            if MaterialClipboard::fits(&text) {
                state.copy_too_large = false;
                ctx.copy_text(text);
            } else {
                state.copy_too_large = true;
            }
        }
        right.add_space(gap);
        let id = Id::unique(StratumId::MaterialsFilter);
        let placeholder = Text::message(StratumMessage::FilterItems);
        TextField::new(id, &mut state.filter, TextFieldKind::Search, placeholder).show(&mut right, Self::SEARCH_WIDTH);

        let room = (right.min_rect().left() - rect.left() - gap).max(0.0);
        let painter = ui.painter();
        let text = <color::Text as ColorToken>::VALUE;
        let heading = subject.heading();
        Draw::text(painter, rect.left_top(), Align2::LEFT_TOP, &heading, Self::HEADING, text, TextFit::Truncate(room));
        if let Some(items) = items {
            let faint = <color::TextFaint as ColorToken>::VALUE;
            let below = rect.left_top() + vec2(0.0, TypeSize::Body.line_height());
            let summary = subject.summary(items);
            Draw::text(painter, below, Align2::LEFT_TOP, &summary, Self::SUMMARY, faint, TextFit::Truncate(room));
        }
    }

    fn show_table(&self, ui: &mut Ui, state: &LibraryUiState) {
        let frame = Surface::frame(SurfaceLevel::Raised, Elevation::Flat).inner_margin(egui::Margin::ZERO);
        let room = ui.available_height();
        frame.show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(room);
            ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
            let inner = (ui.available_width() - 2.0 * Self::ROW_INSET).max(1.0);
            let count = TableColumn { width_fraction: Self::COUNT_WIDTH / inner };
            let stacks = TableColumn { width_fraction: Self::STACKS_WIDTH / inner };
            let item = TableColumn { width_fraction: (1.0 - count.width_fraction - stacks.width_fraction).max(0.0) };
            TableHeader::new(vec![
                (Text::message(StratumMessage::ColumnItem), item),
                (Text::message(StratumMessage::ColumnCount), count),
                (Text::message(StratumMessage::ColumnStacks), stacks),
            ])
            .aligns(&Self::ALIGNS)
            .show(ui);
            if self.rows.is_empty() {
                let message = if state.filter.trim().is_empty() {
                    StratumMessage::MaterialsEmpty
                } else {
                    StratumMessage::NoMaterialsMatch
                };
                ui.add_space(<spacing::M as SpacingToken>::VALUE);
                Caption::new(Text::message(message)).show(ui);
                return;
            }
            let columns = [item, count, stacks];
            let area = ScrollArea::vertical().id_salt(StratumId::MaterialsScroll).auto_shrink([false, false]);
            ScrollBar::show(ui, area, false, |ui| {
                for row in &self.rows {
                    TableRow::new(&columns, TableRowKind::Plain)
                        .size(TableRowSize::Compact)
                        .aligns(&Self::ALIGNS)
                        .show(ui, |index, cell| Self::paint_cell(cell, index, row));
                }
            });
        });
    }

    // Column 0 is the item name at the left, column 1 the count and column 2 the stacks, both at the right.
    fn paint_cell(ui: &mut Ui, index: usize, row: &MaterialRow) {
        let cell = ui.max_rect();
        let painter = ui.painter();
        let text = <color::Text as ColorToken>::VALUE;
        let dim = <color::TextDim as ColorToken>::VALUE;
        match index {
            0 => {
                let name = Text::user(UserText::from_model(row.name.clone()));
                let fit = TextFit::Truncate(cell.width());
                Draw::text(painter, cell.left_center(), Align2::LEFT_CENTER, &name, Self::CELL, text, fit);
            }
            1 => {
                let count = Text::user(UserText::from_model(DigitGroups::of(row.count)));
                Draw::text(painter, cell.right_center(), Align2::RIGHT_CENTER, &count, Self::CELL, text, TextFit::Line);
            }
            _ => {
                let stacks = StackText::of(row.stacks);
                let at = cell.right_center();
                Draw::text(painter, at, Align2::RIGHT_CENTER, &stacks, Self::CELL_DIM, dim, TextFit::Line);
            }
        }
    }
}

impl Default for MaterialsTab {
    fn default() -> Self {
        Self::new()
    }
}
