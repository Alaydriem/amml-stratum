use amml_i18n::text::{Args, Text, UserText};
use amml_ui::draw::{Corner, Draw, TextFit, TextStyle, TypeSize};
use amml_ui::font::FontFace;
use amml_ui::icon::{Icon, IconSize};
use amml_ui::state::InteractionState;
use amml_ui::text::UiText;
use amml_ui::tokens::rule::Elevation;
use amml_ui::tokens::{ColorToken, SpacingToken, color, spacing};
use amml_ui::widget::{
    Carousel, CarouselItem, CarouselMark, Chip, ChipKind, ChipSize, ChipTone, KeyCap, KeyCapLabel, KeyCapSize,
};
use egui::{Align2, Color32, Context, Id, LayerId, Order, Painter, Rect, Vec2, pos2, vec2};
use stratum_core::services::HudInput;
use stratum_core::structs::easy_place::EasyPlaceNotice;
use stratum_core::structs::hud::{HudControl, HudMode, HudState};
use stratum_core::structs::marker::OpacityStep;
use stratum_core::structs::snapshot::HudRow;

use super::StratumId;
use crate::message::StratumMessage;

// The HUD card at the bottom right of the screen: the active marker, its layer readout, its status chips and
// the keys. Quick mode shows the fixed keys; Full mode shows the control carousel. Display only. The texts, the
// chips and every measured width are built again only when the row, the key state or the pixels per point
// change; a frame paints from them.
pub struct HudCard {
    // The row and key state the parts below were built for.
    built: Option<(HudRow, HudState)>,
    // The pixels per point the widths below were measured at; NaN until measured.
    measured_at: f32,
    name: Text,
    subline: Text,
    big: Text,
    detail: Text,
    level: Option<Text>,
    chips: [Chip; 4],
    // The chip rects relative to the chip section's top left.
    chip_rects: [Rect; 4],
    chips_height: f32,
    // The easy place notice line; None while there is no notice to show.
    notice: Option<Text>,
    items: Vec<CarouselItem>,
    current: usize,
    // The key cross: left, up, down and right.
    caps: [KeyCap; 4],
    // The key cross labels: Show, Layer and Opacity.
    labels: [Text; 3],
    middle_width: f32,
    quick_hints: [(KeyCap, Text); 1],
    full_hints: [(KeyCap, Text); 3],
    // The cap and text widths of each hint pair.
    quick_widths: [(f32, f32); 1],
    full_widths: [(f32, f32); 3],
}

impl HudCard {
    /// Paper: Stratum, `Stratum · 16 HUD · quick mode`, node "Build HUD": the card width.
    const WIDTH: f32 = 344.0;
    /// Paper: Stratum, `Stratum · 16 HUD · quick mode`: the gap from the card to the right edge of the screen.
    const RIGHT: f32 = 32.0;
    /// Paper: Stratum, `Stratum · 16 HUD · quick mode`: the gap from the card to the bottom edge of the screen.
    const BOTTOM: f32 = 40.0;
    /// Paper: Stratum, `Stratum · 16 HUD · quick mode`: the padding at the left and right of every section.
    const PAD_X: f32 = 16.0;
    /// Paper: Stratum, `Stratum · 16 HUD · quick mode`, header: the padding above the row.
    const HEADER_TOP: f32 = 14.0;
    /// Paper: Stratum, `Stratum · 16 HUD · quick mode`, header: the padding under the row.
    const HEADER_BOTTOM: f32 = 12.0;
    /// Paper: Stratum, `Stratum · 16 HUD · quick mode`, header: the gap between the pin and the text column.
    const HEADER_GAP: f32 = 10.0;
    /// Paper: Stratum, `Stratum · 16 HUD · quick mode`, header: the line height of the pin.
    const ICON_LINE: f32 = 20.0;
    /// Paper: Stratum, `Stratum · 16 HUD · quick mode`: the gap between two lines of a text column.
    const TEXT_GAP: f32 = 2.0;
    /// Paper: Stratum, `Stratum · 16 HUD · quick mode`, layer readout: the padding above the row.
    const READOUT_TOP: f32 = 6.0;
    /// Paper: Stratum, `Stratum · 16 HUD · quick mode`, layer readout: the padding under the row.
    const READOUT_BOTTOM: f32 = 14.0;
    /// Paper: Stratum, `Stratum · 16 HUD · quick mode`, layer readout: the gap between the big text and the column.
    const READOUT_GAP: f32 = 12.0;
    /// Paper: Stratum, `Stratum · 16 HUD · quick mode`, status chips: the padding above the chips.
    const CHIPS_TOP: f32 = 10.0;
    /// Paper: Stratum, `Stratum · 16 HUD · quick mode`, status chips: the padding under the chips.
    const CHIPS_BOTTOM: f32 = 12.0;
    /// Paper: Stratum, `Stratum · 16 HUD · quick mode`, status chips: the gap between chips and between rows.
    const CHIPS_GAP: f32 = 6.0;
    /// Paper: Stratum, `Stratum · 19 HUD · easy place`, easy place notice: the padding above and under the row.
    const NOTICE_PAD_Y: f32 = 10.0;
    /// Paper: Stratum, `Stratum · 19 HUD · easy place`, easy place notice: the gap between the icon and the text.
    const NOTICE_GAP: f32 = 10.0;
    /// Paper: Stratum, `Stratum · 16 HUD · quick mode`, keys: the padding above the section.
    const KEYS_TOP: f32 = 14.0;
    /// Paper: Stratum, `Stratum · 16 HUD · quick mode`, keys: the padding under the section.
    const KEYS_BOTTOM: f32 = 12.0;
    /// Paper: Stratum, `Stratum · 16 HUD · quick mode`, keys: the gap between the key cross and the mode hint.
    const KEYS_GAP: f32 = 10.0;
    /// Paper: Stratum, `Stratum · 16 HUD · quick mode`, key cross: the gap between the groups.
    const CROSS_GAP: f32 = 16.0;
    /// Paper: Stratum, `Stratum · 16 HUD · quick mode`, key cross: the gap between the caps and the label of a group.
    const GROUP_GAP: f32 = 4.0;
    /// Paper: Stratum, `Stratum · 16 HUD · quick mode`, key cross: the width of the left and right groups.
    const SIDE_WIDTH: f32 = 64.0;
    /// Paper: Stratum, `Stratum · 16 HUD · quick mode`, key cross: the line height of a key label.
    const LABEL_LINE: f32 = 16.0;
    /// Paper: Stratum, `Stratum · 15 HUD · full controls` and `16 HUD · quick mode`: the gap between a small cap
    /// and its text.
    const HINT_GAP: f32 = 5.0;
    /// Paper: Stratum, `Stratum · 15 HUD · full controls`, key hints: the gap between two hint pairs.
    const HINT_PAIR_GAP: f32 = 8.0;
    const NAME: TextStyle = TextStyle::new(TypeSize::Label, FontFace::Semibold);
    const CAPTION: TextStyle = TextStyle::new(TypeSize::Caption, FontFace::Regular);
    const BIG: TextStyle = TextStyle::new(TypeSize::Display, FontFace::Semibold);
    const NOTE: TextStyle = TextStyle::new(TypeSize::Note, FontFace::Regular);

    pub fn new() -> Self {
        let empty = || Text::message(StratumMessage::HudWholeBuild);
        let chip = || Chip::new(ChipKind::Display(ChipTone::Neutral), empty()).size(ChipSize::Hud);
        let arrow = |icon| KeyCap { label: KeyCapLabel::Icon(icon), size: KeyCapSize::Hud, conflict: false };
        let hint = |key, label| {
            let cap =
                KeyCap { label: KeyCapLabel::Text(Text::message(key)), size: KeyCapSize::HudHint, conflict: false };
            (cap, Text::message(label))
        };
        Self {
            built: None,
            measured_at: f32::NAN,
            name: empty(),
            subline: empty(),
            big: empty(),
            detail: empty(),
            level: None,
            chips: [chip(), chip(), chip(), chip()],
            chip_rects: [Rect::NOTHING; 4],
            chips_height: 0.0,
            notice: None,
            items: Vec::with_capacity(HudInput::controls(true).len()),
            current: 0,
            caps: [arrow(Icon::ChevronLeft), arrow(Icon::ChevronUp), arrow(Icon::ChevronDown), arrow(Icon::Chevron)],
            labels: [
                Text::message(StratumMessage::HudShow),
                Text::message(StratumMessage::HudLayer),
                Text::message(StratumMessage::HudOpacity),
            ],
            middle_width: 0.0,
            quick_hints: [hint(StratumMessage::HudModeKey, StratumMessage::HudAllControls)],
            full_hints: [
                hint(StratumMessage::HudChooseKeys, StratumMessage::HudChoose),
                hint(StratumMessage::HudChangeKeys, StratumMessage::HudChange),
                hint(StratumMessage::HudModeKey, StratumMessage::HudQuick),
            ],
            quick_widths: [(0.0, 0.0); 1],
            full_widths: [(0.0, 0.0); 3],
        }
    }

    // Paints the card over the screen and returns its rect.
    pub fn show(&mut self, ctx: &Context, hud: &HudRow, state: &HudState) -> Rect {
        self.rebuild(hud, state);
        if self.measured_at != ctx.pixels_per_point() {
            self.measure(ctx);
        }
        let screen = ctx.content_rect();
        let height = Self::header_height()
            + Self::readout_height()
            + self.chips_height
            + self.notice_height()
            + match state.mode {
                HudMode::Quick => Self::quick_height(),
                HudMode::Full => self.full_height(),
            };
        let rect = Rect::from_min_size(
            pos2(screen.right() - Self::RIGHT - Self::WIDTH, screen.bottom() - Self::BOTTOM - height),
            vec2(Self::WIDTH, height),
        );
        let layer = LayerId::new(Order::Background, Id::unique(StratumId::HudCard));
        let painter = ctx.layer_painter(layer).with_clip_rect(rect);
        let line = <color::LineOverlay as ColorToken>::VALUE;
        Draw::surface(&painter, rect, Corner::Panel, <color::Card as ColorToken>::VALUE, Some(line), Elevation::Flat);
        let mut top = self.paint_header(&painter, rect);
        top = self.paint_readout(&painter, rect, top);
        Self::divider(&painter, rect, top);
        self.paint_chips(&painter, rect, top);
        top += self.chips_height;
        Self::divider(&painter, rect, top);
        top = self.paint_notice(&painter, rect, top);
        match state.mode {
            HudMode::Quick => self.paint_quick(&painter, rect, top),
            HudMode::Full => self.paint_full(&painter, rect, top),
        }
        rect
    }

    fn rebuild(&mut self, hud: &HudRow, state: &HudState) {
        if matches!(&self.built, Some((row, built)) if row == hud && built == state) {
            return;
        }
        self.measured_at = f32::NAN;
        self.name = Text::user(UserText::from_model(hud.marker_name.clone()));
        let name = || UserText::from_model(hud.target_name.clone());
        self.subline = match (&hud.piece_name, hud.position) {
            (Some(piece), _) => {
                Text::format(StratumMessage::HudPiece, Args::new().user("name", UserText::from_model(piece.clone())))
            }
            (None, Some((number, count))) => Text::format(
                StratumMessage::HudPosition,
                Args::new().user("name", name()).uint("number", u64::from(number)).uint("count", u64::from(count)),
            ),
            (None, None) => Text::user(name()),
        };
        let layers = u64::from(hud.layers);
        match hud.layer {
            None if hud.layers == 0 => {
                self.big = Text::user(UserText::from_model(String::new()));
                self.detail = Text::user(UserText::from_model(String::new()));
                self.level = None;
            }
            Some(layer) => {
                self.big = Text::user(UserText::from_model((u64::from(layer) + 1).to_string()));
                self.detail = Text::plural(StratumMessage::HudOfLayers, layers, Args::new());
                self.level =
                    hud.y_level.map(|y| Text::format(StratumMessage::HudYLevel, Args::new().int("y", i64::from(y))));
            }
            None => {
                self.big = Text::message(StratumMessage::HudWholeBuild);
                self.detail = Text::plural(StratumMessage::LayerCount, layers, Args::new());
                self.level = hud.y_range.map(|(low, high)| {
                    let args = Args::new().int("low", i64::from(low)).int("high", i64::from(high));
                    Text::format(StratumMessage::HudYRange, args)
                });
            }
        }
        let [x, y, z] = hud.origin.map(i64::from);
        let degrees = u64::from(hud.rotation.degrees());
        let shown = if hud.shown { StratumMessage::HudShown } else { StratumMessage::HudHidden };
        let dot = if hud.shown { <color::Success as ColorToken>::VALUE } else { <color::TextDim as ColorToken>::VALUE };
        let chip = |text| Chip::new(ChipKind::Display(ChipTone::Neutral), text).size(ChipSize::Hud);
        let easy_place = if hud.easy_place {
            Chip::new(ChipKind::Display(ChipTone::Accent), Text::message(StratumMessage::HudEasyPlace))
                .size(ChipSize::Hud)
                .dot(<color::Accent as ColorToken>::VALUE)
        } else {
            chip(Text::message(StratumMessage::HudEasyPlaceOffChip))
                .off(true)
                .dot(<color::TextFaint as ColorToken>::VALUE)
        };
        self.chips = [
            chip(Text::message(shown)).dot(dot),
            chip(Text::format(StratumMessage::HudOpacityChip, Self::opacity_args(hud))),
            chip(Text::format(
                StratumMessage::HudOriginChip,
                Args::new().int("x", x).int("y", y).int("z", z).uint("degrees", degrees),
            )),
            easy_place,
        ];
        self.notice = match (hud.notice, &hud.notice_item) {
            (Some(EasyPlaceNotice::Missing { .. }), Some(item)) => Some(Text::format(
                StratumMessage::EasyPlaceMissing,
                Args::new().user("block", UserText::from_model(item.readable())),
            )),
            (Some(EasyPlaceNotice::NoNeighbour), _) => Some(Text::message(StratumMessage::EasyPlaceNoNeighbour)),
            (Some(EasyPlaceNotice::Missing { .. }), None) | (None, _) => None,
        };
        let controls = HudInput::carousel(state.control, hud.has_structures);
        self.items.clear();
        self.items.extend(controls.iter().map(|control| Self::item(*control, hud)));
        self.current = controls.iter().position(|control| *control == state.control).unwrap_or(0);
        self.built = Some((hud.clone(), *state));
    }

    // Lays out the chips and measures the key cross and the hints.
    fn measure(&mut self, ctx: &Context) {
        self.measured_at = ctx.pixels_per_point();
        let room = Self::WIDTH - 2.0 * Self::PAD_X;
        let (mut x, mut y) = (0.0_f32, 0.0_f32);
        let height = ChipSize::Hud.height();
        for (chip, slot) in self.chips.iter().zip(self.chip_rects.iter_mut()) {
            let width = chip.desired_width(ctx).min(room);
            if x > 0.0 && x + width > room {
                x = 0.0;
                y += height + Self::CHIPS_GAP;
            }
            *slot = Rect::from_min_size(pos2(x, y), vec2(width, height));
            x += width + Self::CHIPS_GAP;
        }
        self.chips_height = Self::CHIPS_TOP + y + height + Self::CHIPS_BOTTOM;
        let label_width = Self::text_width(ctx, &self.labels[1]);
        self.middle_width = KeyCapSize::Hud.min_width().max(label_width);
        let widths = |(cap, label): &(KeyCap, Text)| (cap.desired_width(ctx), Self::text_width(ctx, label));
        for (pair, slot) in self.quick_hints.iter().zip(self.quick_widths.iter_mut()) {
            *slot = widths(pair);
        }
        for (pair, slot) in self.full_hints.iter().zip(self.full_widths.iter_mut()) {
            *slot = widths(pair);
        }
    }

    fn text_width(ctx: &Context, text: &Text) -> f32 {
        Draw::measure(ctx, &UiText::resolve(ctx, text), Self::CAPTION).x.ceil()
    }

    fn opacity_args(hud: &HudRow) -> Args {
        Args::new().uint("number", u64::from(hud.opacity_step) + 1).uint("count", u64::from(OpacityStep::COUNT))
    }

    fn item(control: HudControl, hud: &HudRow) -> CarouselItem {
        let origin =
            |message, axis: usize| Text::format(message, Args::new().int("value", i64::from(hud.origin[axis])));
        let letter = |message, tone: Color32| CarouselMark::Letter(Text::message(message), tone);
        let (mark, name, value) = match control {
            HudControl::Layer => {
                let value = match hud.layer {
                    Some(layer) => Text::format(
                        StratumMessage::HudStepOf,
                        Args::new().uint("number", u64::from(layer) + 1).uint("count", u64::from(hud.layers)),
                    ),
                    None if hud.layers == 0 => Text::user(UserText::from_model(String::new())),
                    None => Text::message(StratumMessage::HudWholeBuild),
                };
                (CarouselMark::Icon(Icon::Layers), StratumMessage::HudLayer, value)
            }
            HudControl::Opacity => (
                CarouselMark::Icon(Icon::Brightness),
                StratumMessage::HudOpacity,
                Text::format(StratumMessage::HudStepOf, Self::opacity_args(hud)),
            ),
            HudControl::Show => {
                let value = if hud.shown { StratumMessage::HudShown } else { StratumMessage::HudHidden };
                (CarouselMark::Icon(Icon::View), StratumMessage::HudShow, Text::message(value))
            }
            HudControl::Structure => (
                CarouselMark::Icon(Icon::Folder),
                StratumMessage::HudStructure,
                Text::user(UserText::from_model(hud.target_name.clone())),
            ),
            HudControl::MoveX => (
                letter(StratumMessage::HudLetterX, <color::AxisX as ColorToken>::VALUE),
                StratumMessage::HudMoveX,
                origin(StratumMessage::HudOriginX, 0),
            ),
            HudControl::MoveY => (
                letter(StratumMessage::HudLetterY, <color::AxisY as ColorToken>::VALUE),
                StratumMessage::HudMoveY,
                origin(StratumMessage::HudOriginY, 1),
            ),
            HudControl::MoveZ => (
                letter(StratumMessage::HudLetterZ, <color::AxisZ as ColorToken>::VALUE),
                StratumMessage::HudMoveZ,
                origin(StratumMessage::HudOriginZ, 2),
            ),
            HudControl::EasyPlace => {
                let value =
                    if hud.easy_place { StratumMessage::HudEasyPlaceOn } else { StratumMessage::HudEasyPlaceOff };
                (CarouselMark::Icon(Icon::Add), StratumMessage::HudEasyPlace, Text::message(value))
            }
            HudControl::Rotate => (
                CarouselMark::Icon(Icon::Rotate),
                StratumMessage::HudRotate,
                Text::format(
                    StratumMessage::HudDegrees,
                    Args::new().uint("degrees", u64::from(hud.rotation.degrees())),
                ),
            ),
        };
        CarouselItem { mark, name: Text::message(name), value }
    }

    fn header_height() -> f32 {
        Self::HEADER_TOP + Self::header_row() + Self::HEADER_BOTTOM
    }

    fn header_row() -> f32 {
        Self::ICON_LINE.max(Self::NAME.size.line_height() + Self::TEXT_GAP + Self::CAPTION.size.line_height())
    }

    // The height of the notice line; nothing while there is no notice.
    fn notice_height(&self) -> f32 {
        if self.notice.is_some() {
            2.0 * Self::NOTICE_PAD_Y + Self::ICON_LINE.max(Self::NOTE.size.line_height())
        } else {
            0.0
        }
    }

    fn readout_height() -> f32 {
        Self::READOUT_TOP + Self::readout_row() + Self::READOUT_BOTTOM
    }

    fn readout_row() -> f32 {
        let column = Self::NOTE.size.line_height() + Self::TEXT_GAP + Self::CAPTION.size.line_height();
        Self::BIG.size.line_height().max(column)
    }

    fn quick_height() -> f32 {
        Self::KEYS_TOP + Self::cross_height() + Self::KEYS_GAP + KeyCapSize::HudHint.height() + Self::KEYS_BOTTOM
    }

    // The middle group of the key cross: two caps stacked, then the label.
    fn cross_height() -> f32 {
        2.0 * KeyCapSize::Hud.height() + 2.0 * Self::GROUP_GAP + Self::LABEL_LINE
    }

    fn full_height(&self) -> f32 {
        Carousel::new(&self.items, self.current).height() + KeyCapSize::HudHint.height() + Self::KEYS_BOTTOM
    }

    // A 1 px `line-overlay` border across the card, ending at `bottom`.
    fn divider(painter: &Painter, rect: Rect, bottom: f32) {
        let width = <spacing::Divider as SpacingToken>::VALUE;
        Draw::divider(painter, rect.x_range(), bottom - width, <color::LineOverlay as ColorToken>::VALUE);
    }

    // Returns the bottom of the header.
    fn paint_header(&self, painter: &Painter, rect: Rect) -> f32 {
        let centre = rect.top() + Self::HEADER_TOP + Self::header_row() / 2.0;
        let side = IconSize::Regular.points();
        let icon = Rect::from_center_size(pos2(rect.left() + Self::PAD_X + side / 2.0, centre), Vec2::splat(side));
        Icon::Pin.paint(painter, icon, IconSize::Regular, <color::Success as ColorToken>::VALUE);
        let left = icon.right() + Self::HEADER_GAP;
        let fit = TextFit::Truncate((rect.right() - Self::PAD_X - left).max(0.0));
        let column = Self::NAME.size.line_height() + Self::TEXT_GAP + Self::CAPTION.size.line_height();
        let top = centre - column / 2.0;
        let text = <color::Text as ColorToken>::VALUE;
        Draw::text(painter, pos2(left, top), Align2::LEFT_TOP, &self.name, Self::NAME, text, fit);
        let sub_top = top + Self::NAME.size.line_height() + Self::TEXT_GAP;
        let faint = <color::TextFaint as ColorToken>::VALUE;
        Draw::text(painter, pos2(left, sub_top), Align2::LEFT_TOP, &self.subline, Self::CAPTION, faint, fit);
        rect.top() + Self::header_height()
    }

    // Returns the bottom of the readout.
    fn paint_readout(&self, painter: &Painter, rect: Rect, top: f32) -> f32 {
        let centre = top + Self::READOUT_TOP + Self::readout_row() / 2.0;
        let text = <color::Text as ColorToken>::VALUE;
        let at = pos2(rect.left() + Self::PAD_X, centre);
        let big = Draw::text(painter, at, Align2::LEFT_CENTER, &self.big, Self::BIG, text, TextFit::Line);
        let left = big.right() + Self::READOUT_GAP;
        let room = TextFit::Truncate((rect.right() - Self::PAD_X - left).max(0.0));
        let column = Self::NOTE.size.line_height() + Self::TEXT_GAP + Self::CAPTION.size.line_height();
        let column_top = centre - column / 2.0;
        let dim = <color::TextDim as ColorToken>::VALUE;
        Draw::text(painter, pos2(left, column_top), Align2::LEFT_TOP, &self.detail, Self::NOTE, dim, room);
        if let Some(level) = &self.level {
            let level_top = column_top + Self::NOTE.size.line_height() + Self::TEXT_GAP;
            let faint = <color::TextFaint as ColorToken>::VALUE;
            Draw::text(painter, pos2(left, level_top), Align2::LEFT_TOP, level, Self::CAPTION, faint, room);
        }
        top + Self::readout_height()
    }

    fn paint_chips(&self, painter: &Painter, rect: Rect, top: f32) {
        let origin = vec2(rect.left() + Self::PAD_X, top + Self::CHIPS_TOP);
        for (chip, slot) in self.chips.iter().zip(&self.chip_rects) {
            chip.paint(painter, slot.translate(origin), InteractionState::Rest);
        }
    }

    // Paints the easy place notice line from `top` and returns its bottom; `top` itself without a notice.
    fn paint_notice(&self, painter: &Painter, rect: Rect, top: f32) -> f32 {
        let Some(notice) = &self.notice else {
            return top;
        };
        let height = self.notice_height();
        let row = Rect::from_min_size(pos2(rect.left(), top), vec2(rect.width(), height));
        Draw::fill(painter, row, Corner::None, <color::WarningSoft as ColorToken>::VALUE);
        Self::divider(painter, rect, row.bottom());
        let side = IconSize::Regular.points();
        let centre = row.center().y;
        let icon = Rect::from_center_size(pos2(row.left() + Self::PAD_X + side / 2.0, centre), Vec2::splat(side));
        Icon::Warning.paint(painter, icon, IconSize::Regular, <color::Warning as ColorToken>::VALUE);
        let left = icon.right() + Self::NOTICE_GAP;
        let fit = TextFit::Truncate((row.right() - Self::PAD_X - left).max(0.0));
        let text = <color::Text as ColorToken>::VALUE;
        Draw::text(painter, pos2(left, centre), Align2::LEFT_CENTER, notice, Self::NOTE, text, fit);
        row.bottom()
    }

    fn paint_quick(&self, painter: &Painter, rect: Rect, top: f32) {
        let cross_top = top + Self::KEYS_TOP;
        let total = 2.0 * Self::SIDE_WIDTH + 2.0 * Self::CROSS_GAP + self.middle_width;
        let left = rect.center().x - total / 2.0;
        let side_height = KeyCapSize::Hud.height() + Self::GROUP_GAP + Self::LABEL_LINE;
        let side_top = cross_top + (Self::cross_height() - side_height) / 2.0;
        let middle = left + Self::SIDE_WIDTH + Self::CROSS_GAP + self.middle_width / 2.0;
        let [cap_left, cap_up, cap_down, cap_right] = &self.caps;
        let [show, layer, opacity] = &self.labels;
        Self::paint_group(painter, left + Self::SIDE_WIDTH / 2.0, side_top, &[cap_left], show);
        Self::paint_group(painter, middle, cross_top, &[cap_up, cap_down], layer);
        Self::paint_group(painter, left + total - Self::SIDE_WIDTH / 2.0, side_top, &[cap_right], opacity);
        let hint = cross_top + Self::cross_height() + Self::KEYS_GAP;
        Self::paint_hints(painter, rect.center().x, hint, &self.quick_hints, &self.quick_widths);
    }

    // Caps stacked from `top`, centred on `centre`, then the label.
    fn paint_group(painter: &Painter, centre: f32, top: f32, caps: &[&KeyCap], label: &Text) {
        let size = vec2(KeyCapSize::Hud.min_width(), KeyCapSize::Hud.height());
        let mut y = top;
        for cap in caps {
            let rect = Rect::from_min_size(pos2(centre - size.x / 2.0, y), size);
            cap.paint(painter, rect);
            y = rect.bottom() + Self::GROUP_GAP;
        }
        let faint = <color::TextFaint as ColorToken>::VALUE;
        let at = pos2(centre, y + Self::LABEL_LINE / 2.0);
        Draw::text(painter, at, Align2::CENTER_CENTER, label, Self::CAPTION, faint, TextFit::Line);
    }

    fn paint_full(&self, painter: &Painter, rect: Rect, top: f32) {
        let carousel = Carousel::new(&self.items, self.current);
        let row = carousel.paint(painter, pos2(rect.left(), top), Self::WIDTH);
        Self::paint_hints(painter, rect.center().x, row.bottom(), &self.full_hints, &self.full_widths);
    }

    // A centred row of small key caps from `top`, each followed by its text.
    fn paint_hints(painter: &Painter, centre: f32, top: f32, pairs: &[(KeyCap, Text)], widths: &[(f32, f32)]) {
        let gaps = pairs.len().saturating_sub(1) as f32 * Self::HINT_PAIR_GAP;
        let total = widths.iter().map(|(cap, label)| cap + Self::HINT_GAP + label).sum::<f32>() + gaps;
        let height = KeyCapSize::HudHint.height();
        let middle = top + height / 2.0;
        let faint = <color::TextFaint as ColorToken>::VALUE;
        let mut x = centre - total / 2.0;
        for ((cap, label), (cap_width, label_width)) in pairs.iter().zip(widths) {
            let rect = Rect::from_min_size(pos2(x, top), vec2(*cap_width, height));
            cap.paint(painter, rect);
            let at = pos2(rect.right() + Self::HINT_GAP, middle);
            Draw::text(painter, at, Align2::LEFT_CENTER, label, Self::CAPTION, faint, TextFit::Line);
            x = at.x + label_width + Self::HINT_PAIR_GAP;
        }
    }
}

impl Default for HudCard {
    fn default() -> Self {
        Self::new()
    }
}
