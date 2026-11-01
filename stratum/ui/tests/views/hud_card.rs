use amml_ui::font::FontSource;
use amml_ui::theme::Theme;
use egui::epaint::ClippedShape;
use egui::{Context, Pos2, RawInput, Rect, vec2};
use stratum_core::services::HudInput;
use stratum_core::structs::easy_place::EasyPlaceNotice;
use stratum_core::structs::hud::{HudControl, HudMode, HudState};
use stratum_core::structs::marker::Rotation;
use stratum_core::structs::material::ItemId;
use stratum_core::structs::snapshot::HudRow;
use stratum_ui::views::HudCard;

use crate::support::ShapeProbe;

// One egui context that draws the HUD card for a row and a key state.
struct CardPass;

impl CardPass {
    // Layer 7 of 18 of "Main hall", the first of four structures, at origin (-12, 64, 38) turned 90 degrees.
    fn row() -> HudRow {
        HudRow {
            marker_name: "Castle".to_owned(),
            target_name: "Main hall".to_owned(),
            position: Some((1, 4)),
            piece_name: None,
            layers: 18,
            layer: Some(6),
            y_level: Some(71),
            y_range: Some((64, 81)),
            shown: true,
            opacity_step: 2,
            origin: [-12, 64, 38],
            rotation: Rotation::R90,
            has_structures: true,
            easy_place: false,
            notice: None,
            notice_item: None,
        }
    }

    fn quick() -> HudState {
        HudState::new()
    }

    fn full(control: HudControl) -> HudState {
        HudState { mode: HudMode::Full, control }
    }

    // The shapes of the second frame, so fonts have settled, and the card's rect.
    fn draw(row: &HudRow, state: &HudState) -> (Vec<ClippedShape>, Rect) {
        let ctx = Context::default();
        Theme::install(&ctx, FontSource::BuiltIn);
        let mut card = HudCard::new();
        let mut shapes = Vec::new();
        let mut rect = Rect::NOTHING;
        for frame in 0..2 {
            let screen = Rect::from_min_size(Pos2::ZERO, vec2(1920.0, 1080.0));
            let input = RawInput { screen_rect: Some(screen), time: Some(f64::from(frame)), ..RawInput::default() };
            let mut output = ctx.run_ui(input, |ui| rect = card.show(ui.ctx(), row, state));
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
fn card_shows_layer_of_layers() {
    let (shapes, rect) = CardPass::draw(&CardPass::row(), &CardPass::quick());

    assert!(CardPass::has(&shapes, "7"), "the shown layer, counted from 1");
    assert!(CardPass::has(&shapes, "of 18 layers"));
    assert!(CardPass::has(&shapes, "y 71"));
    assert!(CardPass::has(&shapes, "Castle"));
    assert!(CardPass::has(&shapes, "Main hall · 1 of 4"));
    assert_eq!((rect.right(), rect.bottom()), (1920.0 - 32.0, 1080.0 - 40.0), "bottom right of the screen");
    assert_eq!(rect.width(), 344.0);
}

#[test]
fn card_shows_whole_build_after_the_top() {
    let row = HudRow { layer: None, y_level: None, ..CardPass::row() };
    let (shapes, _) = CardPass::draw(&row, &CardPass::quick());

    assert!(CardPass::has(&shapes, "Whole build"));
    assert!(CardPass::has(&shapes, "18 layers"));
    assert!(CardPass::has(&shapes, "y 64 to 81"));
    assert!(!CardPass::has(&shapes, "of 18 layers"));
}

#[test]
fn card_names_the_big_build_piece() {
    let row = HudRow {
        target_name: "Keep".to_owned(),
        position: None,
        piece_name: Some("Gate".to_owned()),
        ..CardPass::row()
    };
    let (shapes, _) = CardPass::draw(&row, &CardPass::quick());

    assert!(CardPass::has(&shapes, "Big build · piece Gate"));
}

#[test]
fn card_chips_show_opacity_step_of_eight_origin_and_rotation() {
    let (shapes, _) = CardPass::draw(&CardPass::row(), &CardPass::quick());

    assert!(CardPass::has(&shapes, "Shown"));
    assert!(CardPass::has(&shapes, "Opacity 3 of 8"));
    assert!(CardPass::has(&shapes, "-12, 64, 38 · 90°"));

    let hidden = HudRow { shown: false, ..CardPass::row() };
    let (shapes, _) = CardPass::draw(&hidden, &CardPass::quick());
    assert!(CardPass::has(&shapes, "Hidden"));
    assert!(!CardPass::has(&shapes, "Shown"));
}

#[test]
fn quick_mode_shows_the_key_labels_and_the_mode_hint() {
    let (shapes, _) = CardPass::draw(&CardPass::row(), &CardPass::quick());

    for label in ["Show", "Layer", "Opacity", "F10", "All controls"] {
        assert!(CardPass::has(&shapes, label), "{label} is shown");
    }
    assert!(!CardPass::has(&shapes, "Quick"));
    for arrow in ["\u{E76B}", "\u{E70E}", "\u{E70D}", "\u{E76C}"] {
        assert!(CardPass::has(&shapes, arrow), "the arrow key caps paint chevrons");
    }
}

#[test]
fn full_mode_shows_the_carousel_at_the_current_control() {
    let (shapes, rect) = CardPass::draw(&CardPass::row(), &CardPass::full(HudControl::Opacity));

    assert!(CardPass::has(&shapes, "Opacity"), "the current control's name");
    assert!(CardPass::has(&shapes, "3 of 8"), "the current control's value");
    assert!(CardPass::has(&shapes, "\u{E793}"), "the opacity mark is the brightness glyph");
    for hint in ["Choose", "Change", "Quick", "← →", "↑ ↓"] {
        assert!(CardPass::has(&shapes, hint), "{hint} is shown");
    }
    assert!(!CardPass::has(&shapes, "All controls"));
    assert_eq!(rect.bottom(), 1080.0 - 40.0);

    let (shapes, _) = CardPass::draw(&CardPass::row(), &CardPass::full(HudControl::MoveX));
    assert!(CardPass::has(&shapes, "Move X"));
    assert!(CardPass::has(&shapes, "Origin x -12"));
    let (shapes, _) = CardPass::draw(&CardPass::row(), &CardPass::full(HudControl::Structure));
    assert!(CardPass::has(&shapes, "Structure"));
    assert!(CardPass::has(&shapes, "Main hall"));
}

#[test]
fn the_easy_place_chip_shows_on_and_off() {
    let on = HudRow { easy_place: true, ..CardPass::row() };
    let (shapes, _) = CardPass::draw(&on, &CardPass::quick());
    assert!(CardPass::has(&shapes, "Easy place"));
    assert!(!CardPass::has(&shapes, "Easy place off"));

    let (shapes, _) = CardPass::draw(&CardPass::row(), &CardPass::quick());
    assert!(CardPass::has(&shapes, "Easy place off"));
    assert!(!CardPass::has(&shapes, "Easy place"));
    let (shapes, _) = CardPass::draw(&CardPass::row(), &CardPass::full(HudControl::Layer));
    assert!(CardPass::has(&shapes, "Easy place off"), "the chip shows in full mode too");
}

#[test]
fn the_carousel_shows_easy_place_after_rotate() {
    let controls = HudInput::carousel(HudControl::Rotate, true);
    let rotate = controls.iter().position(|control| *control == HudControl::Rotate).unwrap();
    assert_eq!(controls.get(rotate + 1), Some(&HudControl::EasyPlace));

    let (shapes, _) = CardPass::draw(&CardPass::row(), &CardPass::full(HudControl::EasyPlace));
    assert!(CardPass::has(&shapes, "Easy place"), "the control name");
    assert!(CardPass::has(&shapes, "Off"));
    assert!(CardPass::has(&shapes, "\u{E710}"), "the mark is the add glyph");
    let on = HudRow { easy_place: true, ..CardPass::row() };
    let (shapes, _) = CardPass::draw(&on, &CardPass::full(HudControl::EasyPlace));
    assert!(CardPass::has(&shapes, "On"));
}

#[test]
fn a_missing_block_notice_names_the_block() {
    let row = HudRow {
        easy_place: true,
        notice: Some(EasyPlaceNotice::Missing { palette_id: 3 }),
        notice_item: Some(ItemId("minecraft:oak_planks".to_owned())),
        ..CardPass::row()
    };
    for state in [CardPass::quick(), CardPass::full(HudControl::EasyPlace)] {
        let (shapes, _) = CardPass::draw(&row, &state);
        assert!(CardPass::has(&shapes, "No Oak Planks in your inventory"));
        assert!(CardPass::has(&shapes, "\u{E7BA}"), "the warning glyph");
    }

    let row = HudRow { notice: Some(EasyPlaceNotice::NoNeighbour), notice_item: None, ..row };
    let (shapes, _) = CardPass::draw(&row, &CardPass::quick());
    assert!(CardPass::has(&shapes, "Nothing to place this block against"));
}

#[test]
fn the_notice_line_is_empty_without_a_notice() {
    let (shapes, plain) = CardPass::draw(&CardPass::row(), &CardPass::quick());
    assert!(!CardPass::has(&shapes, "\u{E7BA}"));
    assert!(!CardPass::has(&shapes, "Nothing to place this block against"));

    let noticed = HudRow { notice: Some(EasyPlaceNotice::NoNeighbour), ..CardPass::row() };
    let (_, with_notice) = CardPass::draw(&noticed, &CardPass::quick());
    assert!(with_notice.height() > plain.height(), "the line takes room only while there is a notice");
    assert_eq!(with_notice.bottom(), plain.bottom());
}

#[test]
fn full_mode_shows_the_control_the_keys_act_on_when_structures_vanish() {
    let row = HudRow { has_structures: false, position: None, ..CardPass::row() };
    let (shapes, _) = CardPass::draw(&row, &CardPass::full(HudControl::Structure));

    assert!(CardPass::has(&shapes, "Structure"), "Up and Down act on Structure, so the card names it");
    assert!(!CardPass::has(&shapes, "Layer"));
}

#[test]
fn a_missing_target_shows_no_layer_readout() {
    let row = HudRow {
        target_name: String::new(),
        position: None,
        layers: 0,
        layer: None,
        y_level: None,
        y_range: None,
        ..CardPass::row()
    };
    let (shapes, _) = CardPass::draw(&row, &CardPass::quick());

    assert!(!CardPass::has(&shapes, "Whole build"));
    assert!(!CardPass::has(&shapes, "0 layers"));
    let (shapes, _) = CardPass::draw(&row, &CardPass::full(HudControl::Layer));
    assert!(!CardPass::has(&shapes, "Whole build"));
}
