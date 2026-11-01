use std::sync::Arc;

use amml_i18n::text::{Args, Text};
use amml_ui::draw::Draw;
use amml_ui::icon::Icon;
use amml_ui::tokens::rule::Elevation;
use amml_ui::tokens::{ColorToken, color};
use amml_ui::widget::{ChipTone, IconButton, PageHeader, PageHeaderLead, Surface, SurfaceLevel};
use egui::{Align, Area, Context, Id, Layout, Margin, Order, Rect, Sense, Ui, UiBuilder, Vec2};
use stratum_core::services::preview::PreviewService;
use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::snapshot::LibrarySnapshot;

use super::dialogs::DialogOutcome;
use super::{DetailView, SidebarView, StratumId};
use crate::message::StratumMessage;
use crate::state::{LibraryUiState, Page, ProjectTab};

// The library menu: a header, the sidebar at the left and the detail column. It draws the snapshot and pushes actions;
// it never changes the library.
pub struct LibraryView {
    sidebar: SidebarView,
    detail: DetailView,
}

impl LibraryView {
    /// Paper: Stratum, `Stratum · 01 Library`, node "Panel": its width on the 1920 by 1080 artboard.
    const PANEL_WIDTH: f32 = 1474.0;
    /// Paper: Stratum, `Stratum · 01 Library`, node "Panel": its height.
    const PANEL_HEIGHT: f32 = 900.0;
    /// Paper: Stratum, `Stratum · 01 Library`, node "Sidebar": its width, with the divider at its right.
    const SIDEBAR_WIDTH: f32 = 470.0;
    /// Paper: Stratum, `Stratum · 01 Library`, node "Sidebar": the padding at the left and the top and bottom.
    const SIDEBAR_PAD_START: f32 = 24.0;
    /// Paper: Stratum, `Stratum · 01 Library`, node "Sidebar": the padding at the right, and the 1 px divider.
    const SIDEBAR_PAD_END: f32 = 21.0;
    /// Paper: Stratum, `Stratum · 01 Library`, node "Detail": the padding at the left.
    const DETAIL_PAD_LEFT: f32 = 28.0;
    /// Paper: Stratum, `Stratum · 01 Library`, node "Detail": the padding at the top.
    const DETAIL_PAD_TOP: f32 = 20.0;
    /// Paper: Stratum, `Stratum · 01 Library`, node "Detail": the padding at the right.
    const DETAIL_PAD_RIGHT: f32 = 24.0;
    /// Paper: Stratum, `Stratum · 01 Library`, node "Detail": the padding at the bottom.
    const DETAIL_PAD_BOTTOM: f32 = 24.0;

    // With a `preview` service the structure card shows the 3D preview it draws; with None, the size box.
    pub fn new(preview: Option<Arc<PreviewService>>) -> Self {
        Self { sidebar: SidebarView::new(), detail: DetailView::new(preview) }
    }

    // Draws the menu over the screen and returns the panel's rect, for the host's input layer.
    pub fn show(
        &mut self,
        ctx: &Context,
        snapshot: &LibrarySnapshot,
        state: &mut LibraryUiState,
        out: &mut Vec<LibraryAction>,
    ) -> Rect {
        state.reconcile(snapshot);
        let now = ctx.input(|input| input.time);
        if let Some(page) = state.pick_flash.observe(snapshot.latest_pick.as_ref(), now) {
            state.page = page;
            state.project_tab = ProjectTab::Structures;
            // an import into a normal project lands on the first row it brought
            if matches!(page, Page::Project(_))
                && let Some(first) = snapshot.latest_pick.as_ref().and_then(|pick| pick.structures.first())
            {
                state.selected_structure = Some(*first);
            }
        }
        state.last_revision = snapshot.revision;
        let screen = ctx.content_rect();
        // the panel keeps its Paper size on a screen that has room, and shrinks to a smaller one
        let size = Vec2::new(Self::PANEL_WIDTH.min(screen.width()), Self::PANEL_HEIGHT.min(screen.height()));
        let panel = Rect::from_center_size(screen.center(), size);
        let area = Area::new(Id::unique(StratumId::Panel))
            .order(Order::Foreground)
            .fixed_pos(panel.min)
            .constrain(false)
            .show(ctx, |ui| {
                Surface::frame(SurfaceLevel::Panel, Elevation::Floating).inner_margin(Margin::ZERO).show(ui, |ui| {
                    ui.set_min_size(size);
                    ui.set_max_size(size);
                    ui.spacing_mut().item_spacing = Vec2::ZERO;
                    self.show_panel(ui, snapshot, state, out);
                });
            });
        if let Some(dialog) = state.dialog.as_mut()
            && dialog.show(ctx, snapshot, out) == DialogOutcome::Closed
        {
            state.dialog = None;
        }
        area.response.rect
    }

    fn show_panel(
        &mut self,
        ui: &mut Ui,
        snapshot: &LibrarySnapshot,
        state: &mut LibraryUiState,
        out: &mut Vec<LibraryAction>,
    ) {
        let mut close = false;
        let header =
            PageHeader::new(PageHeaderLead::Tile(Icon::Apps, ChipTone::Accent), Text::message(StratumMessage::AppName))
                .eyebrow(Text::message(StratumMessage::Eyebrow))
                .meta(vec![
                    Self::count(StratumMessage::ProjectsCount, snapshot.projects.len()),
                    Self::count(StratumMessage::StructuresCount, snapshot.all_count),
                    Self::count(StratumMessage::MarkersInWorldCount, snapshot.markers.len()),
                ])
                .trailing(|ui| {
                    let button = IconButton::new(Icon::Close).tooltip(Text::message(StratumMessage::Close));
                    close = button.show(ui).clicked();
                });
        let header = header.show(ui);
        if close {
            state.close_requested = true;
        }
        let line = <color::Line as ColorToken>::VALUE;
        let painter = ui.painter().clone();
        Draw::divider(&painter, header.rect.x_range(), header.rect.bottom(), line);

        let body = ui.available_rect_before_wrap();
        ui.allocate_rect(body, Sense::hover());
        let sidebar = Rect::from_min_max(
            body.min,
            egui::pos2((body.left() + Self::SIDEBAR_WIDTH).min(body.right()), body.bottom()),
        );
        let detail = Rect::from_min_max(egui::pos2(sidebar.right(), body.top()), body.max);
        painter.vline(sidebar.right(), body.y_range(), Draw::border(line));

        let sidebar_inner = Rect::from_min_max(
            egui::pos2(sidebar.left() + Self::SIDEBAR_PAD_START, sidebar.top() + Self::SIDEBAR_PAD_START),
            egui::pos2(sidebar.right() - Self::SIDEBAR_PAD_END, sidebar.bottom() - Self::SIDEBAR_PAD_START),
        );
        let detail_inner = Rect::from_min_max(
            egui::pos2(detail.left() + Self::DETAIL_PAD_LEFT, detail.top() + Self::DETAIL_PAD_TOP),
            egui::pos2(detail.right() - Self::DETAIL_PAD_RIGHT, detail.bottom() - Self::DETAIL_PAD_BOTTOM),
        );
        let mut sidebar_ui =
            ui.new_child(UiBuilder::new().max_rect(sidebar_inner).layout(Layout::top_down(Align::Min)));
        self.sidebar.show(&mut sidebar_ui, snapshot, state, out);
        let mut detail_ui = ui.new_child(UiBuilder::new().max_rect(detail_inner).layout(Layout::top_down(Align::Min)));
        self.detail.show(&mut detail_ui, snapshot, state, out);
    }

    fn count(message: StratumMessage, count: usize) -> Text {
        Text::plural(message, u64::try_from(count).unwrap_or(u64::MAX), Args::new())
    }
}
