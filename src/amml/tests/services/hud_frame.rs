use std::sync::Arc;

use alaydriem_stratum::services::{FrameRouter, HudFrame, SharedUi};
use amm_client::link::{FrameHandler, FrameInputView, FrameReply};
use amm_core::structs::frame::{FrameInputData, LayerKind};
use amml_ui::font::FontSource;
use amml_ui::theme::Theme;
use egui::{Context, Pos2, RawInput, Rect, vec2};
use stratum_core::structs::hud::HudState;
use stratum_core::structs::marker::Rotation;
use stratum_core::structs::snapshot::{HudRow, LibrarySnapshot};

use crate::support::{CoreSession, Hud, Markers};

// One egui context that runs a frame handler on a layer, as the bridge does on the render thread.
struct HudPass {
    ctx: Context,
    time: f64,
}

impl HudPass {
    fn new() -> Self {
        let ctx = Context::default();
        Theme::install(&ctx, FontSource::BuiltIn);
        Self { ctx, time: 0.0 }
    }

    fn run(&mut self, handler: &mut impl FrameHandler, layer: LayerKind) -> FrameReply {
        self.time += 0.1;
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(1920.0, 1080.0));
        let input = RawInput { screen_rect: Some(screen), time: Some(self.time), ..RawInput::default() };
        let bits = match layer {
            LayerKind::Hud => FrameInputData::LAYER_HUD,
            LayerKind::Overlay => FrameInputData::LAYER_OVERLAY,
        };
        let data = FrameInputData {
            width: 1920,
            height: 1080,
            pixels_per_point: 1.0,
            time_secs: self.time,
            events: &[],
            allowed_layers: bits,
            input_flags: 0,
        };
        let view = FrameInputView::new(data, layer);
        let mut reply = None;
        let mut output = self.ctx.run_ui(input, |ui| reply = Some(handler.frame(ui.ctx(), &view)));
        output.textures_delta.clear();
        reply.unwrap()
    }

    fn with_card() -> LibrarySnapshot {
        let mut snapshot = LibrarySnapshot::empty();
        snapshot.revision = 1;
        snapshot.hud = Some(HudRow {
            marker_name: "Castle".to_owned(),
            target_name: "Main hall".to_owned(),
            position: None,
            piece_name: None,
            layers: 4,
            layer: Some(0),
            y_level: Some(64),
            y_range: Some((64, 67)),
            shown: true,
            opacity_step: 2,
            origin: [0, 64, 0],
            rotation: Rotation::R0,
            has_structures: false,
            easy_place: false,
            notice: None,
            notice_item: None,
        });
        snapshot
    }
}

#[test]
fn hud_frame_draws_only_while_hud_shown() {
    let shared = SharedUi::new_shared(None);
    let mut frame = HudFrame::new(Arc::clone(&shared));
    let mut pass = HudPass::new();
    shared.publish(Arc::new(HudPass::with_card()));

    let hidden = pass.run(&mut frame, LayerKind::Hud);
    shared.publish_hud(Some(HudState::new()));
    let shown = pass.run(&mut frame, LayerKind::Hud);
    let overlay = pass.run(&mut frame, LayerKind::Overlay);
    shared.publish_hud(None);
    let gone = pass.run(&mut frame, LayerKind::Hud);

    assert_eq!(hidden.layer_rect, Rect::NOTHING);
    assert!(shown.layer_rect.is_positive());
    assert_eq!(overlay.layer_rect, Rect::NOTHING, "the card draws on the Hud layer only");
    assert_eq!(gone.layer_rect, Rect::NOTHING);
}

#[test]
fn hud_frame_draws_nothing_without_a_card_row() {
    let shared = SharedUi::new_shared(None);
    let mut frame = HudFrame::new(Arc::clone(&shared));
    shared.publish_hud(Some(HudState::new()));

    assert_eq!(HudPass::new().run(&mut frame, LayerKind::Hud).layer_rect, Rect::NOTHING);
}

#[test]
fn the_router_sends_the_hud_layer_to_the_card_and_the_overlay_to_the_library() {
    let shared = SharedUi::new_shared(None);
    let mut router = FrameRouter::new(Arc::clone(&shared));
    let mut pass = HudPass::new();
    shared.publish(Arc::new(HudPass::with_card()));
    shared.publish_hud(Some(HudState::new()));

    let card = pass.run(&mut router, LayerKind::Hud);
    let closed = pass.run(&mut router, LayerKind::Overlay);
    shared.open();
    let library = pass.run(&mut router, LayerKind::Overlay);

    assert!(card.layer_rect.is_positive());
    assert_eq!(closed.layer_rect, Rect::NOTHING);
    assert!(library.layer_rect.is_positive());
    assert_ne!(card.layer_rect, library.layer_rect);
}

#[test]
fn a_session_with_an_active_marker_draws_the_card() {
    let session = CoreSession::new();
    let mut frame = HudFrame::new(session.shared());
    Markers::place(&session);
    assert!(Hud::shown(&session));

    assert!(HudPass::new().run(&mut frame, LayerKind::Hud).layer_rect.is_positive());

    session.open();
    assert!(!Hud::shown(&session));
    assert_eq!(HudPass::new().run(&mut frame, LayerKind::Hud).layer_rect, Rect::NOTHING);
}
