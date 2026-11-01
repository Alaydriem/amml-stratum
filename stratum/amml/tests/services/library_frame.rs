use std::sync::Arc;

use alaydriem_stratum::services::{LibraryFrame, SharedUi};
use amm_abi::types::AmmLogLevel;
use amm_client::link::{FrameHandler, FrameInputView, FrameReply};
use amm_client::test_support::FakeTable;
use amm_core::structs::frame::{FrameInputData, LayerKind};
use amml_ui::font::FontSource;
use amml_ui::theme::Theme;
use egui::epaint::ClippedShape;
use egui::{Context, Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, Shape, vec2};
use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::library::{ProjectId, ProjectKind, StructureId};
use stratum_core::structs::snapshot::{ImportPick, LibrarySnapshot, ProjectRow, StructureRow};
use stratum_core::structs::structure::StructureSize;
use stratum_ui::state::{Page, PickFlash};

use crate::support::CoreSession;

// One egui context that runs a library frame handler, as the bridge does on the render thread.
struct FramePass {
    ctx: Context,
    frame: LibraryFrame,
    time: f64,
    // The shapes of the last pass, so a test can click where a label is painted.
    shapes: Vec<ClippedShape>,
}

impl FramePass {
    fn new(shared: Arc<SharedUi>) -> Self {
        let ctx = Context::default();
        Theme::install(&ctx, FontSource::BuiltIn);
        Self { ctx, frame: LibraryFrame::new(shared), time: 0.0, shapes: Vec::new() }
    }

    fn run(&mut self, layer: LayerKind, events: Vec<Event>) -> FrameReply {
        self.time += 0.1;
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(1920.0, 1080.0));
        let input = RawInput { screen_rect: Some(screen), time: Some(self.time), events, ..RawInput::default() };
        let data = FrameInputData {
            width: 1920,
            height: 1080,
            pixels_per_point: 1.0,
            time_secs: self.time,
            events: &[],
            allowed_layers: FrameInputData::LAYER_OVERLAY,
            input_flags: FrameInputData::ALLOW_POINTER | FrameInputData::ALLOW_KEYBOARD,
        };
        let view = FrameInputView::new(data, layer);
        let Self { ctx, frame, .. } = self;
        let mut reply = None;
        let mut output = ctx.run_ui(input, |ui| reply = Some(frame.frame(ui.ctx(), &view)));
        output.textures_delta.clear();
        self.shapes = output.shapes;
        reply.unwrap()
    }

    // Moves to the first painted text that contains `label`, presses and releases there, then settles.
    fn click(&mut self, label: &str) {
        let pos = self.shapes.iter().find_map(|clipped| Self::find(&clipped.shape, label)).expect("label is painted");
        self.run(LayerKind::Overlay, vec![Event::PointerMoved(pos)]);
        for pressed in [true, false] {
            let event =
                Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE };
            self.run(LayerKind::Overlay, vec![event]);
        }
        self.run(LayerKind::Overlay, Vec::new());
    }

    fn find(shape: &Shape, label: &str) -> Option<Pos2> {
        match shape {
            Shape::Text(text) if text.galley.text().contains(label) => {
                Some(text.galley.rect.translate(text.pos.to_vec2()).center())
            }
            Shape::Vec(shapes) => shapes.iter().find_map(|inner| Self::find(inner, label)),
            _ => None,
        }
    }

    fn project_names(shared: &SharedUi) -> Vec<String> {
        shared.snapshot().projects.iter().map(|project| project.name.clone()).collect()
    }

    fn escape() -> Vec<Event> {
        vec![Event::Key {
            key: Key::Escape,
            physical_key: Some(Key::Escape),
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }]
    }
}

#[test]
fn actions_sent_by_the_frame_are_applied_on_the_next_tick() {
    let session = CoreSession::new();
    let shared = session.shared();

    let dropped = shared.send(&mut vec![LibraryAction::CreateProject("Castle".to_owned())]);
    assert_eq!(dropped, 0);
    assert!(FramePass::project_names(&shared).is_empty());
    session.tick(&CoreSession::in_world());

    assert_eq!(FramePass::project_names(&shared), ["Castle"]);
}

#[test]
fn action_queue_full_drops_and_logs() {
    let session = CoreSession::new();
    let shared = session.shared();
    let mut actions: Vec<_> =
        (0..=SharedUi::ACTION_CAPACITY).map(|n| LibraryAction::CreateProject(format!("Project {n}"))).collect();
    assert_eq!(actions.len(), 257);

    let dropped = shared.send(&mut actions);

    assert_eq!(dropped, 1);
    assert!(actions.is_empty());
    let lines = CoreSession::logged("library actions dropped");
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].level, AmmLogLevel::Warn as u8);
    session.tick(&CoreSession::in_world());
    assert_eq!(shared.snapshot().projects.len(), SharedUi::ACTION_CAPACITY);
}

#[test]
fn frame_draws_only_on_the_overlay_while_open() {
    let shared = SharedUi::new_shared(None);
    let mut pass = FramePass::new(Arc::clone(&shared));

    let closed = pass.run(LayerKind::Overlay, Vec::new());
    shared.open();
    let hud = pass.run(LayerKind::Hud, Vec::new());
    let overlay = pass.run(LayerKind::Overlay, Vec::new());

    assert_eq!(closed.layer_rect, Rect::NOTHING);
    assert_eq!(hud.layer_rect, Rect::NOTHING);
    assert!(overlay.layer_rect.is_positive());
}

#[test]
fn escape_closes_the_library_when_no_dialog_is_open() {
    let shared = SharedUi::new_shared(None);
    let mut pass = FramePass::new(Arc::clone(&shared));
    shared.open();
    pass.run(LayerKind::Overlay, Vec::new());

    pass.run(LayerKind::Overlay, FramePass::escape());

    assert!(!shared.is_shown());
    assert_eq!(pass.run(LayerKind::Overlay, Vec::new()).layer_rect, Rect::NOTHING);
}

#[test]
fn an_open_dialog_takes_escape_and_the_library_stays_shown() {
    let shared = SharedUi::new_shared(None);
    let mut pass = FramePass::new(Arc::clone(&shared));
    shared.open();
    pass.run(LayerKind::Overlay, Vec::new());
    pass.run(LayerKind::Overlay, Vec::new());
    pass.click("New project");
    pass.run(LayerKind::Overlay, Vec::new());

    pass.run(LayerKind::Overlay, FramePass::escape());
    pass.run(LayerKind::Overlay, Vec::new());
    assert!(shared.is_shown());

    // with the dialog gone, the next Escape closes the library
    pass.run(LayerKind::Overlay, FramePass::escape());
    assert!(!shared.is_shown());
}

#[test]
fn a_library_closed_by_escape_draws_again_when_reopened() {
    let shared = SharedUi::new_shared(None);
    let mut pass = FramePass::new(Arc::clone(&shared));
    shared.open();
    pass.run(LayerKind::Overlay, Vec::new());
    pass.run(LayerKind::Overlay, FramePass::escape());
    assert!(shared.take_close_request());
    shared.close();

    shared.open();

    assert!(pass.run(LayerKind::Overlay, Vec::new()).layer_rect.is_positive());
}

#[test]
fn frame_with_a_fake_manager_runs_through_the_bridge() {
    let session = CoreSession::new();
    session.open();

    assert_eq!(FakeTable::run_frame(), 0);
    assert!(session.shared().is_shown());
}

// A library with one project holding one structure, and a pick that put that structure there.
struct PickedLibrary;

impl PickedLibrary {
    fn snapshot() -> LibrarySnapshot {
        let row = StructureRow {
            id: StructureId(10),
            name: "Main hall".to_owned(),
            size: StructureSize::new(8, 8, 8),
            block_count: 100,
            project: Some(ProjectId(7)),
            grid_cell: None,
            origin: None,
            file_name: String::new(),
        };
        let mut pick = ImportPick::new(1, Some(ProjectId(7)));
        pick.structures = vec![StructureId(10)];
        let mut snapshot = LibrarySnapshot::empty();
        snapshot.revision = 1;
        snapshot.projects = vec![ProjectRow {
            id: ProjectId(7),
            name: "Manor".to_owned(),
            kind: ProjectKind::Normal,
            structures: vec![row],
        }];
        snapshot.all_count = 1;
        snapshot.latest_pick = Some(pick);
        snapshot
    }
}

#[test]
fn a_pick_seen_once_does_not_move_the_page_or_flash_on_a_later_open() {
    let shared = SharedUi::new_shared(None);
    let mut pass = FramePass::new(Arc::clone(&shared));
    shared.publish(Arc::new(PickedLibrary::snapshot()));
    shared.open();
    pass.run(LayerKind::Overlay, Vec::new());
    assert_eq!(pass.frame.state().page, Page::Project(ProjectId(7)));
    assert!(pass.frame.state().pick_flash.strength(StructureId(10), pass.time) > 0.0);
    for _ in 0..(PickFlash::FLASH_SECONDS * 10.0) as usize + 2 {
        pass.run(LayerKind::Overlay, Vec::new());
    }
    shared.close();

    shared.open();
    pass.run(LayerKind::Overlay, Vec::new());

    assert_eq!(pass.frame.state().page, Page::All);
    assert_eq!(pass.frame.state().pick_flash.strength(StructureId(10), pass.time), 0.0);
}
