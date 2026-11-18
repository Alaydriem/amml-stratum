use std::sync::Arc;

use amml_ui::font::FontSource;
use amml_ui::theme::Theme;
use amml_ui::tokens::{ColorToken, color};
use egui::epaint::ClippedShape;
use egui::{Context, Pos2, RawInput, Rect, vec2};
use stratum_core::structs::build::{BuildPhase, BuildStopReason};
use stratum_core::structs::hud::HudState;
use stratum_core::structs::marker::Rotation;
use stratum_core::structs::snapshot::{BuildRow, HudRow};
use stratum_ui::views::HudCard;

use crate::support::{BuildFixture, ShapeProbe};

// One egui context that draws the HUD card of a marker with a build row, as on the build card Paper sheets.
struct BuildPass;

impl BuildPass {
    fn marker() -> HudRow {
        HudRow {
            marker_name: "Castle".to_owned(),
            target_name: "Main hall".to_owned(),
            position: None,
            piece_name: None,
            layers: 18,
            layer: None,
            y_level: None,
            y_range: Some((64, 81)),
            shown: true,
            opacity_step: 2,
            origin: [-12, 64, 38],
            rotation: Rotation::R0,
            has_structures: true,
            easy_place: false,
            notice: None,
            notice_item: None,
        }
    }

    fn draw(row: BuildRow) -> (Vec<ClippedShape>, Rect) {
        let mut state = HudState::new();
        state.build = Some(Arc::new(row));
        let ctx = Context::default();
        Theme::install(&ctx, FontSource::BuiltIn);
        let mut card = HudCard::new();
        let mut shapes = Vec::new();
        let mut rect = Rect::NOTHING;
        for frame in 0..2 {
            let screen = Rect::from_min_size(Pos2::ZERO, vec2(1920.0, 1080.0));
            let input = RawInput { screen_rect: Some(screen), time: Some(f64::from(frame)), ..RawInput::default() };
            let mut output = ctx.run_ui(input, |ui| rect = card.show(ui.ctx(), &Self::marker(), &state));
            output.textures_delta.clear();
            shapes = output.shapes;
        }
        (shapes, rect)
    }

    fn has(shapes: &[ClippedShape], text: &str) -> bool {
        !ShapeProbe::exact_rects(shapes, text).is_empty()
    }
}

#[test]
fn the_hud_card_shows_phase_layer_blocks_and_time() {
    let (shapes, rect) = BuildPass::draw(BuildFixture::running(BuildPhase::Building));
    assert!(BuildPass::has(&shapes, "Main hall"), "the structure being built");
    assert!(BuildPass::has(&shapes, "Building"));
    assert!(BuildPass::has(&shapes, "7"), "the big layer number");
    assert!(BuildPass::has(&shapes, "Layer 7 of 18"));
    assert!(BuildPass::has(&shapes, "1,640 of 4,212 blocks"));
    assert!(BuildPass::has(&shapes, "Time left 8 min 12 s"));
    assert!(BuildPass::has(&shapes, "4 blocks/s"));
    assert!(BuildPass::has(&shapes, "Pause"));
    assert!(BuildPass::has(&shapes, "Stop"));
    assert!(!BuildPass::has(&shapes, "Castle"), "the build card takes the place of the marker card");
    assert_eq!((rect.right(), rect.bottom()), (1920.0 - 32.0, 1080.0 - 40.0), "bottom right of the screen");
    assert_eq!(rect.width(), 344.0);
}

#[test]
fn the_progress_bar_fills_by_the_blocks_placed() {
    let (shapes, _) = BuildPass::draw(BuildFixture::running(BuildPhase::Building));
    let fill = ShapeProbe::fill_rects(&shapes, <color::Accent as ColorToken>::VALUE);
    let track = ShapeProbe::fill_rects(&shapes, <color::BgField as ColorToken>::VALUE);
    let fill = fill.iter().find(|rect| rect.height() == 4.0).expect("the filled part");
    let track = track.iter().find(|rect| rect.height() == 4.0).expect("the track");
    let ratio = fill.width() / track.width();
    assert!((ratio - 1640.0 / 4212.0).abs() < 0.01, "{ratio}");
}

#[test]
fn the_card_shows_resume_and_asks_for_a_second_stop_press() {
    let (shapes, _) = BuildPass::draw(BuildFixture::running(BuildPhase::Paused));
    assert!(BuildPass::has(&shapes, "Paused"));
    assert!(BuildPass::has(&shapes, "Resume"));
    assert!(!BuildPass::has(&shapes, "Pause"));
    assert!(BuildPass::has(&shapes, "Stop"));

    let asking = BuildRow { confirm_stop: true, ..BuildFixture::running(BuildPhase::Paused) };
    let (shapes, _) = BuildPass::draw(asking);
    assert!(BuildPass::has(&shapes, "Press Down again to stop"));
    assert!(!BuildPass::has(&shapes, "Stop"));
    let danger = <color::Danger as ColorToken>::VALUE;
    assert_eq!(ShapeProbe::exact_colour(&shapes, "Press Down again to stop"), Some(danger));
}

#[test]
fn the_phase_takes_its_colour() {
    let colour = |phase, name| {
        let (shapes, _) = BuildPass::draw(BuildFixture::running(phase));
        ShapeProbe::exact_colour(&shapes, name)
    };
    assert_eq!(colour(BuildPhase::Clearing, "Clearing"), Some(<color::Accent as ColorToken>::VALUE));
    assert_eq!(colour(BuildPhase::Building, "Building"), Some(<color::Success as ColorToken>::VALUE));
    assert_eq!(colour(BuildPhase::Paused, "Paused"), Some(<color::Warning as ColorToken>::VALUE));
    let fluids = colour(BuildPhase::Fluids, "Placing water and lava");
    assert!(fluids.is_some(), "the fluids phase is named");
    assert_ne!(fluids, colour(BuildPhase::Building, "Building"), "the fluids phase has a colour of its own");
}

#[test]
fn a_stopped_build_says_why_and_has_no_keys() {
    let row = BuildFixture::running(BuildPhase::Stopped(BuildStopReason::MarkerChanged));
    let (shapes, _) = BuildPass::draw(row);
    assert!(BuildPass::has(&shapes, "The build stopped because the marker changed."));
    assert!(!BuildPass::has(&shapes, "Pause"));
    assert!(!BuildPass::has(&shapes, "Stop"));
}

#[test]
fn the_marker_card_shows_while_the_panel_is_open() {
    let (shapes, _) = BuildPass::draw(BuildFixture::ready());
    assert!(BuildPass::has(&shapes, "Castle"));
    assert!(!BuildPass::has(&shapes, "Layer 0 of 18"));
}

#[test]
fn the_marker_card_shows_when_the_build_row_is_gone_after_an_end() {
    let ctx = Context::default();
    Theme::install(&ctx, FontSource::BuiltIn);
    let mut card = HudCard::new();
    let draw = |card: &mut HudCard, state: &HudState| {
        let mut shapes = Vec::new();
        for frame in 0..2 {
            let screen = Rect::from_min_size(Pos2::ZERO, vec2(1920.0, 1080.0));
            let input = RawInput { screen_rect: Some(screen), time: Some(f64::from(frame)), ..RawInput::default() };
            let mut output = ctx.run_ui(input, |ui| {
                card.show(ui.ctx(), &BuildPass::marker(), state);
            });
            output.textures_delta.clear();
            shapes = output.shapes;
        }
        shapes
    };
    let mut state = HudState::new();
    state.build = Some(Arc::new(BuildFixture::running(BuildPhase::Stopped(BuildStopReason::Operator))));
    let shapes = draw(&mut card, &state);
    assert!(BuildPass::has(&shapes, "You stopped the build."));
    assert!(!BuildPass::has(&shapes, "Castle"));

    state.build = None;
    let shapes = draw(&mut card, &state);
    assert!(BuildPass::has(&shapes, "Castle"), "the marker card is back");
    assert!(!BuildPass::has(&shapes, "You stopped the build."));
}
