use amml_i18n::text::{Args, Text, UserText};
use amml_ui::draw::{Draw, TextFit, TextStyle, TypeSize};
use amml_ui::font::FontFace;
use amml_ui::icon::Icon;
use amml_ui::state::DisabledReason;
use amml_ui::text::UiText;
use amml_ui::tokens::rule::Elevation;
use amml_ui::tokens::{ColorToken, SpacingToken, color, spacing};
use amml_ui::widget::{
    Avatar, AvatarSize, Banner, BannerTone, ButtonKind, Caption, EmptyState, IconButton, Leading, ListRow,
    ListRowResponse, ScrollBar, SectionHeader, Surface, SurfaceLevel, TabRow, TextButton, Toggle, ValueRow,
};
use egui::{Align, Align2, Id, Key, Layout, Rect, ScrollArea, Sense, Ui, UiBuilder, pos2, vec2};
use stratum_core::structs::action::{LibraryAction, MaterialScope};
use stratum_core::structs::layout::{BigBuildLayout, LayoutIssue};
use stratum_core::structs::library::{ProjectKind, StructureId};
use stratum_core::structs::marker::MarkerTarget;
use stratum_core::structs::snapshot::{LibrarySnapshot, ProjectRow, StructureRow};

use super::{
    CardFooter, FactsPanel, GridEditor, ImportButton, MarkerActions, MaterialsTab, PageTitle, ProjectMenu, StratumId,
    StructurePreview,
};
use crate::message::StratumMessage;
use crate::presenter::{DigitGroups, MaterialsSubject, PieceOffset};
use crate::state::{LibraryUiState, Page, ProjectTab};

// The page of one project: its title and Big build switch, then the Structures and Materials tabs.
pub struct ProjectPage {
    materials: MaterialsTab,
}

impl ProjectPage {
    const SWITCH_LABEL: TextStyle = TextStyle::new(TypeSize::Label, FontFace::Semibold);
    /// Paper: Stratum (p-E-0), `Stratum · 06 Big build switch + layout ok`, node "Structure list": its width, 360 px with `flex-shrink: 0`.
    const LIST_WIDTH: f32 = 360.0;
    /// Paper: Stratum (p-E-0), `Stratum · 06 Big build switch + layout ok`, node "Split": the `gap` between the list and the preview pane, 20 px.
    const COLUMN_GAP: f32 = 20.0;
    const SWITCH_HELP: TextStyle = TextStyle::new(TypeSize::Caption, FontFace::Regular);

    pub fn new() -> Self {
        Self { materials: MaterialsTab::new() }
    }

    pub fn show(
        &mut self,
        ui: &mut Ui,
        snapshot: &LibrarySnapshot,
        project: &ProjectRow,
        state: &mut LibraryUiState,
        out: &mut Vec<LibraryAction>,
        preview: Option<&mut StructurePreview>,
    ) {
        let blocks = project.structures.iter().fold(0u64, |sum, row| sum.saturating_add(row.block_count));
        let used = Self::marker_count(snapshot, project);
        let used_text = if used == 0 {
            Text::message(StratumMessage::NoMarkersYet)
        } else {
            Text::plural(StratumMessage::UsedByMarkers, used, Args::new())
        };
        let meta = vec![
            Text::plural(StratumMessage::StructuresCount, Self::count(project.structures.len()), Args::new()),
            Text::plural(
                StratumMessage::BlocksCount,
                blocks,
                Args::new().user("blocks", UserText::from_model(DigitGroups::of(blocks))),
            ),
            used_text,
        ];
        let title = PageTitle::new(Text::user(UserText::from_model(project.name.clone()))).meta(meta);
        if project.structures.is_empty() {
            // a marker for a project with no structure shows nothing, so both marker items wait
            let nothing = Some(Text::message(StratumMessage::ProjectEmptyTitle));
            let mut placed = None;
            title.show_beside(ui, |ui| placed = ProjectMenu::show(ui, snapshot, project, nothing, state), |_| {});
            out.extend(placed);
            ui.add_space(PageTitle::SECTION_GAP);
            Self::show_empty(ui, snapshot, project, out);
            return;
        }
        let blocked = Self::blocked(project, Self::layout_of(snapshot, project));
        let place_reason = MarkerActions::place_reason(snapshot, MarkerTarget::Project(project.id), blocked.clone());
        let mut placed = None;
        let beside = |ui: &mut Ui| placed = ProjectMenu::show(ui, snapshot, project, blocked, state);
        title.show_beside(ui, beside, |ui| {
            Self::big_build(ui, snapshot, project, out);
            ui.add_space(<spacing::L as SpacingToken>::VALUE);
            if ImportButton::build(snapshot, ButtonKind::Secondary).show(ui).clicked() {
                out.push(LibraryAction::RequestFilePicker(Some(project.id)));
            }
            if project.kind == ProjectKind::Normal {
                ui.add_space(<spacing::L as SpacingToken>::VALUE);
                Self::place_project_marker(ui, snapshot, project, place_reason, out);
            }
        });
        out.extend(placed);

        ui.add_space(PageTitle::SECTION_GAP);
        let mut selected = match state.project_tab {
            ProjectTab::Structures => 0,
            ProjectTab::Materials => 1,
        };
        let tabs = vec![Text::message(StratumMessage::TabStructures), Text::message(StratumMessage::TabMaterials)];
        TabRow::new(Id::unique(StratumId::ProjectTabs), &mut selected, tabs).show(ui);
        state.project_tab = if selected == 0 { ProjectTab::Structures } else { ProjectTab::Materials };

        ui.add_space(PageTitle::SECTION_GAP);
        match state.project_tab {
            ProjectTab::Structures => Self::show_structures(ui, snapshot, project, state, out, preview),
            ProjectTab::Materials => {
                if let Some(subject) = MaterialsSubject::of(snapshot, MaterialScope::Project(project.id)) {
                    self.materials.show(ui, snapshot, state, &subject, out);
                }
            }
        }
    }

    // The first step of a project with no structures: import into it.
    fn show_empty(ui: &mut Ui, snapshot: &LibrarySnapshot, project: &ProjectRow, out: &mut Vec<LibraryAction>) {
        let empty = EmptyState::new(
            Icon::Folder,
            Text::message(StratumMessage::ProjectEmptyTitle),
            Text::message(StratumMessage::ProjectEmptyBody),
        )
        .action(ImportButton::build(snapshot, ButtonKind::Primary));
        if empty.show(ui).is_some_and(|action| action.clicked()) {
            out.push(LibraryAction::RequestFilePicker(Some(project.id)));
        }
    }

    fn show_structures(
        ui: &mut Ui,
        snapshot: &LibrarySnapshot,
        project: &ProjectRow,
        state: &mut LibraryUiState,
        out: &mut Vec<LibraryAction>,
        preview: Option<&mut StructurePreview>,
    ) {
        let layout = Self::layout_of(snapshot, project);
        Self::show_layout_banner(ui, layout);
        let area = ui.available_rect_before_wrap();
        let list = Rect::from_min_max(area.min, pos2((area.min.x + Self::LIST_WIDTH).min(area.max.x), area.max.y));
        let card = Rect::from_min_max(pos2(list.max.x + Self::COLUMN_GAP, area.min.y), area.max);
        ui.allocate_rect(area, Sense::hover());
        let mut list_ui = ui.new_child(UiBuilder::new().max_rect(list).layout(Layout::top_down(Align::Min)));
        let selected = Self::selected(project, state);
        Self::show_list(&mut list_ui, snapshot, project, layout, selected, state, out);
        let mut card_ui = ui.new_child(UiBuilder::new().max_rect(card).layout(Layout::top_down(Align::Min)));
        Self::show_card(&mut card_ui, snapshot, project, layout, state, out, preview);
    }

    // The layout of a big build project; a normal project has none.
    fn layout_of<'a>(
        snapshot: &'a LibrarySnapshot,
        project: &ProjectRow,
    ) -> Option<&'a Result<BigBuildLayout, LayoutIssue>> {
        if project.kind == ProjectKind::BigBuild {
            snapshot.layouts.iter().find(|(id, _)| *id == project.id).map(|(_, layout)| layout)
        } else {
            None
        }
    }

    // Why the project's marker buttons wait: a big build laid out on the grid with pieces that have no cell yet.
    fn blocked(project: &ProjectRow, layout: Option<&Result<BigBuildLayout, LayoutIssue>>) -> Option<Text> {
        let unplaced = project.structures.iter().filter(|row| row.grid_cell.is_none()).count();
        (Self::uses_grid(layout) && unplaced > 0)
            .then(|| Text::plural(StratumMessage::PiecesNotPlaced, Self::count(unplaced), Args::new()))
    }

    // The banner over a big build: the pieces line up, or the saved positions do not fit.
    fn show_layout_banner(ui: &mut Ui, layout: Option<&Result<BigBuildLayout, LayoutIssue>>) {
        let banner = match layout {
            Some(Ok(_)) => Banner::new(BannerTone::Success, Text::message(StratumMessage::PiecesLineUp))
                .body(Text::message(StratumMessage::PiecesLineUpBody)),
            None => return,
            Some(Err(issue)) => Self::issue_banner(issue),
        };
        banner.show(ui);
        ui.add_space(PageTitle::SECTION_GAP);
    }

    // The banner for a layout that failed: its title and a body that says why.
    fn issue_banner(issue: &LayoutIssue) -> Banner<'static> {
        let (tone, title, body) = match issue {
            LayoutIssue::NoPieces => {
                (BannerTone::Info, StratumMessage::LayoutNoPiecesTitle, StratumMessage::LayoutNoPiecesBody)
            }
            LayoutIssue::MissingOrigin(_) => {
                (BannerTone::Warning, StratumMessage::SavedPositionsDoNotFit, StratumMessage::LayoutMissingOriginBody)
            }
            LayoutIssue::Overlap(..) => (
                BannerTone::Warning,
                StratumMessage::SavedPositionsDoNotFit,
                StratumMessage::SavedPositionsDoNotFitBody,
            ),
            LayoutIssue::GridIncomplete(_) => {
                (BannerTone::Warning, StratumMessage::SavedPositionsDoNotFit, StratumMessage::LayoutGridIncompleteBody)
            }
            LayoutIssue::TooLarge => {
                (BannerTone::Warning, StratumMessage::LayoutTooLargeTitle, StratumMessage::LayoutTooLargeBody)
            }
        };
        Banner::new(tone, Text::message(title)).body(Text::message(body))
    }

    fn show_list(
        ui: &mut Ui,
        snapshot: &LibrarySnapshot,
        project: &ProjectRow,
        layout: Option<&Result<BigBuildLayout, LayoutIssue>>,
        selected: Option<StructureId>,
        state: &mut LibraryUiState,
        out: &mut Vec<LibraryAction>,
    ) {
        let area = ScrollArea::vertical().id_salt(StratumId::StructureListScroll).auto_shrink([false, false]);
        ScrollBar::show(ui, area, false, |ui| {
            let now = ui.input(|input| input.time);
            for row in &project.structures {
                let subtitle = Self::subtitle(project, layout, row);
                let strength = state.pick_flash.strength(row.id, now);
                let target = MarkerTarget::Structure(row.id);
                let place = (project.kind == ProjectKind::Normal).then(|| MarkerActions::row_action(snapshot, target));
                let open = selected == Some(row.id);
                let response = Self::structure_row(ui, row, subtitle, (strength, open), place);
                if strength > 0.0 {
                    ui.ctx().request_repaint();
                }
                if state.pick_flash.take_scroll(row.id) {
                    response.row.scroll_to_me(Some(Align::Center));
                }
                if response.action {
                    out.extend(MarkerActions::place(snapshot, target));
                } else if response.row.clicked() {
                    // a big build has no card of one structure, so its row click opens the page
                    let enter = open && response.row.has_focus() && ui.input(|input| input.key_pressed(Key::Enter));
                    if project.kind == ProjectKind::BigBuild || response.row.double_clicked() || enter {
                        state.page = Page::Structure(row.id);
                    } else {
                        state.selected_structure = Some(row.id);
                        ui.ctx().request_repaint();
                    }
                }
            }
        });
    }

    // The card beside the list: the build layout or the grid of a big build, then the project's actions.
    fn show_card(
        ui: &mut Ui,
        snapshot: &LibrarySnapshot,
        project: &ProjectRow,
        layout: Option<&Result<BigBuildLayout, LayoutIssue>>,
        state: &mut LibraryUiState,
        out: &mut Vec<LibraryAction>,
        preview: Option<&mut StructurePreview>,
    ) {
        if project.kind == ProjectKind::Normal
            && let Some(row) = state.resolve_selection(&project.structures)
        {
            let height = ui.max_rect().height();
            let area = ScrollArea::vertical().id_salt(StratumId::ProjectCardScroll).auto_shrink([false, false]);
            ScrollBar::show(ui, area, false, |ui| {
                out.extend(FactsPanel::show(ui, snapshot, row, state, height, preview));
            });
            return;
        }
        let grid = Self::uses_grid(layout);
        let blocked = Self::blocked(project, layout);
        let height = ui.max_rect().height();
        let area = ScrollArea::vertical().id_salt(StratumId::ProjectCardScroll).auto_shrink([false, false]);
        ScrollBar::show(ui, area, false, |ui| {
            let bottom = ui.cursor().top() + height;
            let frame = Surface::frame(SurfaceLevel::Raised, Elevation::Flat);
            let inset = CardFooter::inset(&frame);
            frame.show(ui, |ui| {
                ui.set_width(ui.available_width());
                if let Some(Ok(built)) = layout {
                    Self::layout_facts(ui, project, built);
                }
                if Self::from_grid(layout, project) {
                    ui.add_space(<spacing::M as SpacingToken>::VALUE);
                    Self::grid_editor(ui, snapshot, project, out);
                } else if grid {
                    Self::grid_editor(ui, snapshot, project, out);
                }
                CardFooter::show(ui, bottom, inset, |ui| {
                    out.extend(MarkerActions::show(ui, snapshot, MarkerTarget::Project(project.id), blocked, state));
                });
            });
        });
    }

    // The structure the list marks and the card shows on a normal project page. A big build has none.
    fn selected(project: &ProjectRow, state: &mut LibraryUiState) -> Option<StructureId> {
        if project.kind != ProjectKind::Normal {
            return None;
        }
        state.resolve_selection(&project.structures).map(|row| row.id)
    }

    // The grid the pieces are dropped into, with its heading and note.
    fn grid_editor(ui: &mut Ui, snapshot: &LibrarySnapshot, project: &ProjectRow, out: &mut Vec<LibraryAction>) {
        SectionHeader::new(Text::message(StratumMessage::GridHeading)).divider(false).show(ui);
        let id = Id::unique(StratumId::GridPlacer).with(project.id.0);
        GridEditor::show(ui, id, snapshot, &project.structures, out);
        ui.add_space(<spacing::M as SpacingToken>::VALUE);
        Caption::new(Text::message(StratumMessage::GridNote)).show(ui);
    }

    // Whether a layout that lines up came from the grid: every piece has a cell. Such a grid stays editable, so a
    // wrong drop can be moved.
    fn from_grid(layout: Option<&Result<BigBuildLayout, LayoutIssue>>, project: &ProjectRow) -> bool {
        matches!(layout, Some(Ok(_)))
            && !project.structures.is_empty()
            && project.structures.iter().all(|row| row.grid_cell.is_some())
    }

    // Whether the pieces are laid out on the grid: the saved positions are missing or do not fit.
    fn uses_grid(layout: Option<&Result<BigBuildLayout, LayoutIssue>>) -> bool {
        matches!(
            layout,
            Some(Err(LayoutIssue::MissingOrigin(_) | LayoutIssue::Overlap(..) | LayoutIssue::GridIncomplete(_)))
        )
    }

    // The "BUILD LAYOUT" facts of a big build that lines up.
    fn layout_facts(ui: &mut Ui, project: &ProjectRow, layout: &BigBuildLayout) {
        SectionHeader::new(Text::message(StratumMessage::BuildLayout)).divider(false).show(ui);
        let blocks = project.structures.iter().fold(0u64, |sum, row| sum.saturating_add(row.block_count));
        let source = if project.structures.iter().all(|row| row.grid_cell.is_some()) {
            StratumMessage::LayoutFromGrid
        } else {
            StratumMessage::LayoutFromSavedPositions
        };
        let size = Args::new()
            .uint("x", u64::from(layout.size.x))
            .uint("y", u64::from(layout.size.y))
            .uint("z", u64::from(layout.size.z));
        let number = |value: u64| Text::user(UserText::from_model(DigitGroups::of(value)));
        let facts = [
            (StratumMessage::LayoutLabel, Text::message(source)),
            (StratumMessage::FactPieces, number(Self::count(layout.pieces.len()))),
            (StratumMessage::FactBuildSize, Text::format(StratumMessage::SizeDims, size)),
            (StratumMessage::FactLayers, number(u64::from(layout.size.y))),
            (StratumMessage::FactBlocks, number(blocks)),
        ];
        for (label, value) in facts {
            ValueRow::new(Text::message(label), value).show(ui);
        }
    }

    // The line under a piece's name: where it sits in the layout, its grid cell, or its size.
    fn subtitle(
        project: &ProjectRow,
        layout: Option<&Result<BigBuildLayout, LayoutIssue>>,
        row: &StructureRow,
    ) -> Text {
        let size = Args::new()
            .uint("x", u64::from(row.size.x))
            .uint("y", u64::from(row.size.y))
            .uint("z", u64::from(row.size.z));
        if let Some(Ok(built)) = layout
            && let Some(piece) = built.pieces.iter().find(|piece| piece.structure == row.id)
        {
            let at = UserText::from_model(PieceOffset::of(piece.offset));
            return Text::format(StratumMessage::PieceAt, size.user("at", at));
        }
        if Self::uses_grid(layout) {
            return match row.grid_cell {
                Some(cell) => {
                    let number = GridEditor::cell_number(&project.structures, cell);
                    Text::format(StratumMessage::PieceCell, size.uint("number", number))
                }
                None => Text::format(StratumMessage::PieceNotPlaced, size),
            };
        }
        Text::format(
            StratumMessage::StructureSubtitle,
            size.user("blocks", UserText::from_model(DigitGroups::of(row.block_count))),
        )
    }

    fn structure_row(
        ui: &mut Ui,
        row: &StructureRow,
        subtitle: Text,
        (flash, selected): (f32, bool),
        place: Option<IconButton>,
    ) -> ListRowResponse {
        let name = UserText::from_model(row.name.clone());
        let mut list_row = ListRow::new(Avatar::new(&name, AvatarSize::Regular), Text::user(name))
            .subtitle(subtitle)
            .flash(flash)
            .selected(selected);
        if let Some(button) = place {
            list_row = list_row.action(button);
        }
        list_row.show(ui)
    }

    // The button that places a marker cycling through the structures of the project.
    fn place_project_marker(
        ui: &mut Ui,
        snapshot: &LibrarySnapshot,
        project: &ProjectRow,
        reason: Option<Text>,
        out: &mut Vec<LibraryAction>,
    ) {
        let mut button = TextButton::new(ButtonKind::Outline, Text::message(StratumMessage::PlaceProjectMarker))
            .leading(Leading::Icon(Icon::Pin));
        if let Some(reason) = reason {
            button = button.enabled(false, DisabledReason::new(reason));
        }
        if button.show(ui).clicked() {
            out.extend(MarkerActions::place(snapshot, MarkerTarget::Project(project.id)));
        }
    }

    // The switch, then its label and help at the left of it. The layout is right to left.
    fn big_build(ui: &mut Ui, snapshot: &LibrarySnapshot, project: &ProjectRow, out: &mut Vec<LibraryAction>) {
        let mut on = project.kind == ProjectKind::BigBuild;
        let reason = DisabledReason::message(StratumMessage::ReadOnlyReason);
        let changed = Toggle::new(&mut on).enabled(!snapshot.library_read_only, reason).show(ui).changed();
        if changed {
            let kind = if on { ProjectKind::BigBuild } else { ProjectKind::Normal };
            out.push(LibraryAction::SetProjectKind(project.id, kind));
        }
        ui.add_space(<spacing::M as SpacingToken>::VALUE);
        let label = Text::message(StratumMessage::BigBuild);
        let help = Text::message(StratumMessage::BigBuildHelp);
        let widest = [(&label, Self::SWITCH_LABEL), (&help, Self::SWITCH_HELP)]
            .iter()
            .map(|(text, style)| Draw::measure(ui.ctx(), &UiText::resolve(ui.ctx(), text), *style).x)
            .fold(0.0, f32::max);
        let height = TypeSize::Label.line_height() + TypeSize::Caption.line_height();
        let (rect, _) = ui.allocate_exact_size(vec2(widest, height), Sense::hover());
        let painter = ui.painter();
        let text = <color::Text as ColorToken>::VALUE;
        let faint = <color::TextFaint as ColorToken>::VALUE;
        Draw::text(painter, rect.right_top(), Align2::RIGHT_TOP, &label, Self::SWITCH_LABEL, text, TextFit::Line);
        let below = rect.right_top() + vec2(0.0, TypeSize::Label.line_height());
        Draw::text(painter, below, Align2::RIGHT_TOP, &help, Self::SWITCH_HELP, faint, TextFit::Line);
    }

    // Markers that show this project or one of its structures.
    fn marker_count(snapshot: &LibrarySnapshot, project: &ProjectRow) -> u64 {
        let used = snapshot.markers.iter().filter(|marker| match marker.target {
            MarkerTarget::Project(id) => id == project.id,
            MarkerTarget::Structure(id) => project.structures.iter().any(|row| row.id == id),
        });
        Self::count(used.count())
    }

    fn count(len: usize) -> u64 {
        u64::try_from(len).unwrap_or(u64::MAX)
    }
}

impl Default for ProjectPage {
    fn default() -> Self {
        Self::new()
    }
}
