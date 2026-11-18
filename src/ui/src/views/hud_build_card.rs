use std::sync::Arc;

use amml_i18n::text::{Args, Text, UserText};
use amml_ui::draw::{Corner, Draw, TextFit};
use amml_ui::icon::{Icon, IconSize};
use amml_ui::state::InteractionState;
use amml_ui::text::UiText;
use amml_ui::tokens::rule::Elevation;
use amml_ui::tokens::{ColorToken, SpacingToken, color, spacing};
use amml_ui::widget::{Chip, ChipKind, ChipSize, ChipTone, KeyCap, KeyCapLabel, KeyCapSize};
use egui::{Align2, Color32, Context, Id, LayerId, Order, Painter, Rect, Vec2, pos2, vec2};
use stratum_core::structs::build::{BuildPhase, BuildStopReason};
use stratum_core::structs::snapshot::BuildRow;

use super::{HudCard, StratumId};
use crate::message::StratumMessage;
use crate::presenter::{BuildTime, DigitGroups};

// The HUD card of a build, at the bottom right of the screen where the marker card is: the structure and the phase,
// the layer, the blocks placed with a progress bar, the time left and the rate, and the keys that pause and stop.
// Display only. The texts, the chips and their widths are built again only when the row or the pixels per point
// change; a frame paints from them.
pub struct HudBuildCard {
    // The row the parts below were built for.
    built: Option<Arc<BuildRow>>,
    // The pixels per point the widths below were measured at; NaN until measured.
    measured_at: f32,
    name: Text,
    phase: Text,
    // The colour of the phase text and of the dot of the time chip.
    tone: Color32,
    big: Text,
    layer: Text,
    blocks: Text,
    // The share of the blocks placed, from 0 to 1.
    progress: f32,
    chips: Vec<Chip>,
    // The chip rects relative to the chip section's top left.
    chip_rects: Vec<Rect>,
    chips_height: f32,
    // The key hints: Up and Down, each with its text; empty once the build has ended.
    hints: Vec<(KeyCap, Text)>,
    hint_tones: Vec<Color32>,
    // The cap and text widths of each hint pair.
    hint_widths: Vec<(f32, f32)>,
}

impl HudBuildCard {
    // The chips of a card: the time left and the rate.
    const CHIPS: usize = 2;
    // The key hints of a running build: Up and Down.
    const HINTS: usize = 2;

    pub fn new() -> Self {
        let empty = || Text::message(StratumMessage::BuildPhaseBuilding);
        Self {
            built: None,
            measured_at: f32::NAN,
            name: empty(),
            phase: empty(),
            tone: <color::TextFaint as ColorToken>::VALUE,
            big: empty(),
            layer: empty(),
            blocks: empty(),
            progress: 0.0,
            chips: Vec::with_capacity(Self::CHIPS),
            chip_rects: Vec::with_capacity(Self::CHIPS),
            chips_height: 0.0,
            hints: Vec::with_capacity(Self::HINTS),
            hint_tones: Vec::with_capacity(Self::HINTS),
            hint_widths: Vec::with_capacity(Self::HINTS),
        }
    }

    // True for a row the card shows: a build that runs or has ended. A row that waits in the settings panel has no
    // card.
    pub fn shows(row: &BuildRow) -> bool {
        !matches!(row.phase, BuildPhase::Probing | BuildPhase::Ready)
    }

    // Paints the card over the screen and returns its rect.
    pub fn show(&mut self, ctx: &Context, row: &Arc<BuildRow>) -> Rect {
        self.rebuild(row);
        if self.measured_at != ctx.pixels_per_point() {
            self.measure(ctx);
        }
        let screen = ctx.content_rect();
        let height = HudCard::header_height() + Self::readout_height() + self.chips_height + self.keys_height();
        let rect = Rect::from_min_size(
            pos2(screen.right() - HudCard::RIGHT - HudCard::WIDTH, screen.bottom() - HudCard::BOTTOM - height),
            vec2(HudCard::WIDTH, height),
        );
        let layer = LayerId::new(Order::Background, Id::unique(StratumId::HudCard));
        let painter = ctx.layer_painter(layer).with_clip_rect(rect);
        let line = <color::LineOverlay as ColorToken>::VALUE;
        Draw::surface(&painter, rect, Corner::Panel, <color::Card as ColorToken>::VALUE, Some(line), Elevation::Flat);
        let mut top = self.paint_header(&painter, rect);
        top = self.paint_readout(&painter, rect, top);
        HudCard::divider(&painter, rect, top);
        self.paint_chips(&painter, rect, top);
        top += self.chips_height;
        if !self.hints.is_empty() {
            HudCard::divider(&painter, rect, top);
            let hints_top = top + HudCard::KEYS_TOP;
            HudCard::paint_hints(
                &painter,
                rect.center().x,
                hints_top,
                &self.hints,
                &self.hint_widths,
                &self.hint_tones,
            );
        }
        rect
    }

    fn rebuild(&mut self, row: &Arc<BuildRow>) {
        if self.built.as_ref().is_some_and(|built| **built == **row) {
            return;
        }
        self.measured_at = f32::NAN;
        self.name = Text::user(UserText::from_model(row.structure_name.clone()));
        self.phase = Self::phase_text(row.phase);
        self.tone = Self::phase_tone(row.phase);
        self.big = Text::user(UserText::from_model(row.layer.to_string()));
        let layers = Args::new().uint("layer", u64::from(row.layer)).uint("layers", u64::from(row.layers));
        self.layer = Text::format(StratumMessage::BuildLayer, layers);
        let blocks = Args::new()
            .user("placed", UserText::from_model(DigitGroups::of(row.placed)))
            .user("total", UserText::from_model(DigitGroups::of(row.total)));
        self.blocks = Text::format(StratumMessage::BuildBlocks, blocks);
        self.progress = if row.total == 0 { 0.0 } else { (row.placed as f32 / row.total as f32).clamp(0.0, 1.0) };
        self.chips.clear();
        let chip = |text| Chip::new(ChipKind::Display(ChipTone::Neutral), text).size(ChipSize::Hud);
        if row.phase.is_running() {
            self.chips.push(chip(BuildTime::left(row.eta_seconds)).dot(self.tone));
        }
        let rate = Args::new().uint("rate", u64::from(row.settings.blocks_per_second));
        self.chips.push(chip(Text::format(StratumMessage::BuildRateChip, rate)));
        self.hints.clear();
        self.hint_tones.clear();
        if row.phase.is_running() {
            self.push_hints(row);
        }
        self.built = Some(Arc::clone(row));
    }

    // Up pauses or resumes; Down stops, and asks for a second press once the first came.
    fn push_hints(&mut self, row: &BuildRow) {
        let faint = <color::TextFaint as ColorToken>::VALUE;
        let cap =
            |key| KeyCap { label: KeyCapLabel::Text(Text::message(key)), size: KeyCapSize::HudHint, conflict: false };
        let up = if row.phase == BuildPhase::Paused { StratumMessage::BuildResume } else { StratumMessage::BuildPause };
        let (down, down_tone) = if row.confirm_stop {
            (StratumMessage::BuildStopConfirm, <color::Danger as ColorToken>::VALUE)
        } else {
            (StratumMessage::BuildStop, faint)
        };
        self.hints.push((cap(StratumMessage::BuildKeyUp), Text::message(up)));
        self.hints.push((cap(StratumMessage::BuildKeyDown), Text::message(down)));
        self.hint_tones.push(faint);
        self.hint_tones.push(down_tone);
    }

    fn phase_text(phase: BuildPhase) -> Text {
        let message = match phase {
            BuildPhase::Clearing => StratumMessage::BuildPhaseClearing,
            BuildPhase::Building => StratumMessage::BuildPhaseBuilding,
            BuildPhase::Fluids => StratumMessage::BuildPhaseFluids,
            BuildPhase::Paused => StratumMessage::BuildPhasePaused,
            BuildPhase::Done => StratumMessage::BuildPhaseDone,
            BuildPhase::Stopped(reason) => Self::stop_message(reason),
            BuildPhase::Probing | BuildPhase::Ready => StratumMessage::BuildProbing,
        };
        Text::message(message)
    }

    fn stop_message(reason: BuildStopReason) -> StratumMessage {
        match reason {
            BuildStopReason::Operator => StratumMessage::BuildStoppedOperator,
            BuildStopReason::LeftWorld => StratumMessage::BuildStoppedLeftWorld,
            BuildStopReason::DimensionChanged => StratumMessage::BuildStoppedDimensionChanged,
            BuildStopReason::ConditionLost => StratumMessage::BuildStoppedConditionLost,
            BuildStopReason::MarkerChanged => StratumMessage::BuildStoppedMarkerChanged,
            BuildStopReason::StructureDeleted => StratumMessage::BuildStoppedStructureDeleted,
            BuildStopReason::SendFailed => StratumMessage::BuildStoppedSendFailed,
        }
    }

    // Paper: Stratum, sheets 23, 23b, 23c and 26. Placing water and lava has `--color-track-fov`, a token of the
    // studio kit that `amml-ui` does not export; `--color-axis-z`, the nearest blue of the kit, stands in for it.
    fn phase_tone(phase: BuildPhase) -> Color32 {
        match phase {
            BuildPhase::Clearing => <color::Accent as ColorToken>::VALUE,
            BuildPhase::Building | BuildPhase::Done => <color::Success as ColorToken>::VALUE,
            BuildPhase::Fluids => <color::AxisZ as ColorToken>::VALUE,
            BuildPhase::Paused => <color::Warning as ColorToken>::VALUE,
            BuildPhase::Stopped(_) => <color::Danger as ColorToken>::VALUE,
            BuildPhase::Probing | BuildPhase::Ready => <color::TextFaint as ColorToken>::VALUE,
        }
    }

    // Lays out the chips and measures the key hints.
    fn measure(&mut self, ctx: &Context) {
        self.measured_at = ctx.pixels_per_point();
        let room = HudCard::WIDTH - 2.0 * HudCard::PAD_X;
        let (mut x, mut y) = (0.0_f32, 0.0_f32);
        let height = ChipSize::Hud.height();
        self.chip_rects.clear();
        for chip in &self.chips {
            let width = chip.desired_width(ctx).min(room);
            if x > 0.0 && x + width > room {
                x = 0.0;
                y += height + HudCard::CHIPS_GAP;
            }
            self.chip_rects.push(Rect::from_min_size(pos2(x, y), vec2(width, height)));
            x += width + HudCard::CHIPS_GAP;
        }
        self.chips_height = HudCard::CHIPS_TOP + y + height + HudCard::CHIPS_BOTTOM;
        self.hint_widths.clear();
        for (cap, label) in &self.hints {
            let text = Draw::measure(ctx, &UiText::resolve(ctx, label), HudCard::CAPTION).x.ceil();
            self.hint_widths.push((cap.desired_width(ctx), text));
        }
    }

    // The keys section: its padding round one row of hints; nothing once the build has ended.
    fn keys_height(&self) -> f32 {
        if self.hints.is_empty() {
            0.0
        } else {
            HudCard::KEYS_TOP + KeyCapSize::HudHint.height() + HudCard::KEYS_BOTTOM
        }
    }

    // The progress bar, the same `--spacing-xs` high as its fill.
    fn bar_height() -> f32 {
        <spacing::Xs as SpacingToken>::VALUE
    }

    // The text column of the readout: the layer line, the blocks line and the bar, 2 px apart.
    fn column_height() -> f32 {
        let lines = HudCard::NOTE.size.line_height() + HudCard::CAPTION.size.line_height();
        lines + 2.0 * HudCard::TEXT_GAP + Self::bar_height()
    }

    fn readout_row() -> f32 {
        HudCard::BIG.size.line_height().max(Self::column_height())
    }

    fn readout_height() -> f32 {
        HudCard::READOUT_TOP + Self::readout_row() + HudCard::READOUT_BOTTOM
    }

    // Returns the bottom of the header.
    fn paint_header(&self, painter: &Painter, rect: Rect) -> f32 {
        let centre = rect.top() + HudCard::HEADER_TOP + HudCard::header_row() / 2.0;
        let side = IconSize::Regular.points();
        let icon = Rect::from_center_size(pos2(rect.left() + HudCard::PAD_X + side / 2.0, centre), Vec2::splat(side));
        Icon::Pin.paint(painter, icon, IconSize::Regular, <color::Success as ColorToken>::VALUE);
        let left = icon.right() + HudCard::HEADER_GAP;
        let fit = TextFit::Truncate((rect.right() - HudCard::PAD_X - left).max(0.0));
        let column = HudCard::NAME.size.line_height() + HudCard::TEXT_GAP + HudCard::CAPTION.size.line_height();
        let top = centre - column / 2.0;
        let text = <color::Text as ColorToken>::VALUE;
        Draw::text(painter, pos2(left, top), Align2::LEFT_TOP, &self.name, HudCard::NAME, text, fit);
        let phase_top = top + HudCard::NAME.size.line_height() + HudCard::TEXT_GAP;
        Draw::text(painter, pos2(left, phase_top), Align2::LEFT_TOP, &self.phase, HudCard::CAPTION, self.tone, fit);
        rect.top() + HudCard::header_height()
    }

    // The layer number, then the layer line, the blocks line and the bar. Returns the bottom of the readout.
    fn paint_readout(&self, painter: &Painter, rect: Rect, top: f32) -> f32 {
        let centre = top + HudCard::READOUT_TOP + Self::readout_row() / 2.0;
        let text = <color::Text as ColorToken>::VALUE;
        let at = pos2(rect.left() + HudCard::PAD_X, centre);
        let big = Draw::text(painter, at, Align2::LEFT_CENTER, &self.big, HudCard::BIG, text, TextFit::Line);
        let left = big.right() + HudCard::READOUT_GAP;
        let width = (rect.right() - HudCard::PAD_X - left).max(0.0);
        let room = TextFit::Truncate(width);
        let column_top = centre - Self::column_height() / 2.0;
        let dim = <color::TextDim as ColorToken>::VALUE;
        Draw::text(painter, pos2(left, column_top), Align2::LEFT_TOP, &self.layer, HudCard::NOTE, dim, room);
        let blocks_top = column_top + HudCard::NOTE.size.line_height() + HudCard::TEXT_GAP;
        let faint = <color::TextFaint as ColorToken>::VALUE;
        Draw::text(painter, pos2(left, blocks_top), Align2::LEFT_TOP, &self.blocks, HudCard::CAPTION, faint, room);
        let bar_top = blocks_top + HudCard::CAPTION.size.line_height() + HudCard::TEXT_GAP;
        let track = Rect::from_min_size(pos2(left, bar_top), vec2(width, Self::bar_height()));
        Draw::fill(painter, track, Corner::Full, <color::BgField as ColorToken>::VALUE);
        let filled = Rect::from_min_size(track.min, vec2((width * self.progress).round(), Self::bar_height()));
        Draw::fill(painter, filled, Corner::Full, <color::Accent as ColorToken>::VALUE);
        top + Self::readout_height()
    }

    fn paint_chips(&self, painter: &Painter, rect: Rect, top: f32) {
        let origin = vec2(rect.left() + HudCard::PAD_X, top + HudCard::CHIPS_TOP);
        for (chip, slot) in self.chips.iter().zip(&self.chip_rects) {
            chip.paint(painter, slot.translate(origin), InteractionState::Rest);
        }
    }
}

impl Default for HudBuildCard {
    fn default() -> Self {
        Self::new()
    }
}
