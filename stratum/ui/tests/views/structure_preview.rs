use amml_ui::font::FontSource;
use amml_ui::theme::Theme;
use egui::{
    CentralPanel, Context, Event, Modifiers, MouseWheelSource, MouseWheelUnit, PointerButton, Pos2, RawInput, Rect,
    ScrollArea, TouchPhase, Vec2,
};
use stratum_core::config::PreviewConfig;
use stratum_core::services::preview::PreviewService;
use stratum_core::structs::preview::{OrbitCamera, PreviewSource};
use stratum_core::structs::snapshot::StructureRow;
use stratum_ui::views::StructurePreview;

use crate::support::SnapshotFixture;

// One pane inside a scrolling page that is taller than the screen, with synthetic pointer input.
struct PaneSession {
    ctx: Context,
    preview: StructurePreview,
    row: StructureRow,
    time: f64,
    pane: Rect,
    offset: f32,
}

impl PaneSession {
    const ABOVE: f32 = 40.0;
    const PANE_HEIGHT: f32 = 300.0;
    const BELOW: f32 = 3000.0;

    fn new() -> Self {
        let ctx = Context::default();
        Theme::install(&ctx, FontSource::BuiltIn);
        let source = PreviewSource::new(None, std::env::temp_dir().join("stratum-none"));
        let service = PreviewService::new_shared(source, &PreviewConfig::default());
        let row = SnapshotFixture::library().projects[0].structures[0].clone();
        let mut session =
            Self { ctx, preview: StructurePreview::new(service), row, time: 0.0, pane: Rect::NOTHING, offset: 0.0 };
        for _ in 0..3 {
            session.frame(Vec::new());
        }
        session
    }

    fn frame(&mut self, events: Vec<Event>) {
        self.time += 0.1;
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
            time: Some(self.time),
            events,
            ..RawInput::default()
        };
        let Self { ctx, preview, row, pane, offset, .. } = self;
        let mut output = ctx.run_ui(input, |ui| {
            CentralPanel::default().show(ui, |ui| {
                let shown = ScrollArea::vertical().show(ui, |ui| {
                    ui.add_space(Self::ABOVE);
                    let top = ui.cursor().top();
                    preview.show(ui, row, Self::PANE_HEIGHT);
                    *pane = Rect::from_min_max(Pos2::new(ui.min_rect().left(), top), ui.cursor().left_top());
                    ui.add_space(Self::BELOW);
                });
                *offset = shown.state.clamped_offset().y;
            });
        });
        output.textures_delta.clear();
    }

    fn inside(&self) -> Pos2 {
        Pos2::new(self.pane.center().x, self.pane.top() + Self::PANE_HEIGHT / 2.0)
    }

    fn drag(&mut self, delta: Vec2) {
        let from = self.inside();
        self.frame(vec![Event::PointerMoved(from)]);
        self.frame(vec![Self::button(from, true)]);
        for step in 1..=4 {
            self.frame(vec![Event::PointerMoved(from + delta * (step as f32 / 4.0))]);
        }
        self.frame(vec![Self::button(from + delta, false)]);
        self.frame(Vec::new());
    }

    fn wheel(&mut self, at: Pos2, lines: f32) {
        self.frame(vec![Event::PointerMoved(at)]);
        self.frame(vec![Event::MouseWheel {
            unit: MouseWheelUnit::Line,
            delta: Vec2::new(0.0, lines),
            phase: TouchPhase::Move,
            source: MouseWheelSource::Unknown,
            modifiers: Modifiers::NONE,
        }]);
        for _ in 0..10 {
            self.frame(Vec::new());
        }
    }

    fn button(pos: Pos2, pressed: bool) -> Event {
        Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
    }
}

#[test]
fn a_drag_across_the_pane_turns_the_camera() {
    let mut session = PaneSession::new();
    let start = session.preview.camera();
    session.drag(Vec2::new(-40.0, 20.0));
    let turned = session.preview.camera();
    assert!(turned.yaw > start.yaw, "a drag to the left turns the yaw up: {start:?} to {turned:?}");
    assert!(turned.pitch > start.pitch, "a drag down turns the pitch up: {start:?} to {turned:?}");
    assert_eq!(turned.zoom, start.zoom);
}

#[test]
fn the_wheel_over_the_pane_zooms_and_the_page_does_not_scroll() {
    let mut session = PaneSession::new();
    let at = session.inside();
    session.wheel(at, 1.0);
    assert!(session.preview.camera().zoom > 1.0, "zoom {}", session.preview.camera().zoom);
    session.wheel(at, -1.0);
    session.wheel(at, -1.0);
    assert!(session.preview.camera().zoom < 1.0, "zoom {}", session.preview.camera().zoom);
    assert_eq!(session.offset, 0.0, "the page scrolled under the pane");
}

#[test]
fn the_wheel_beside_the_pane_scrolls_the_page_and_keeps_the_zoom() {
    let mut session = PaneSession::new();
    let below = Pos2::new(session.pane.center().x, session.pane.bottom() + 100.0);
    session.wheel(below, -3.0);
    assert!(session.offset > 0.0, "the page did not scroll");
    assert_eq!(session.preview.camera(), OrbitCamera::start());
}
