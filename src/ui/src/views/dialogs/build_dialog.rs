use amml_i18n::text::{Args, Text, UserText};
use amml_ui::draw::{Corner, Draw, TextFit, TextStyle, TypeSize};
use amml_ui::font::FontFace;
use amml_ui::icon::{Icon, IconSize};
use amml_ui::state::Selection;
use amml_ui::tokens::{ColorToken, SpacingToken, color, spacing};
use amml_ui::widget::{Caption, Checkbox, DragField, DragUnit, SectionHeader};
use egui::{Align, Align2, Context, Id, Layout, Rect, Sense, Ui, pos2, vec2};
use stratum_core::structs::action::{BuildAction, LibraryAction};
use stratum_core::structs::build::{BuildNotice, BuildPhase, BuildSettings};
use stratum_core::structs::library::StructureId;
use stratum_core::structs::snapshot::{BuildRow, LibrarySnapshot, StructureRow};

use super::{DialogForm, DialogOutcome, FormAnswer};
use crate::message::StratumMessage;
use crate::presenter::{BuildTime, DigitGroups};
use crate::views::StratumId;

// The settings panel of a build: the rate with the time it takes, whether the clear takes water and lava, a notice
// about the last Start, and the blocks the game cannot place. The values are edited here and sent to the build as they
// change; Start sends the start. The dialog closes when the build runs, ends or is no longer allowed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildDialog {
    structure: StructureId,
    // False until the row of the structure arrives and gives the values below.
    loaded: bool,
    // The rate in hundredths of a block a second, so a slow drag adds up before the rate is rounded.
    rate: u32,
    clear_water: bool,
    clear_lava: bool,
    // Start was sent, so a build that ends closes the panel.
    started: bool,
}

impl BuildDialog {
    /// Paper: Stratum, `Stratum · 22 Build dialog`, node "Setting row · Blocks per second": the width of the label.
    const LABEL_WIDTH: f32 = 150.0;
    /// Paper: Stratum, `Stratum · 22 Build dialog`, node "Drag field": the width of the rate field.
    const FIELD_WIDTH: f32 = 120.0;
    /// The rate change for each point dragged on the "Drag field", so the range of 1 to 60 takes about 80 points.
    const RATE_STEP: f64 = 0.75;
    /// Paper: Stratum, `Stratum · 22 Build dialog`, node "Notice · keep fluids": the gap between the icon and the
    /// text, its `gap` of 8 px (`--spacing-s`).
    const NOTICE_GAP: f32 = <spacing::S as SpacingToken>::VALUE;
    const HUNDREDTHS: f64 = 100.0;
    const LABEL: TextStyle = TextStyle::new(TypeSize::Note, FontFace::Regular);
    const VALUE: TextStyle = TextStyle::new(TypeSize::Note, FontFace::Regular);

    // The panel of `structure`, whose row has not arrived yet.
    pub fn new(structure: StructureId) -> Self {
        let rate = u32::from(BuildSettings::default().blocks_per_second) * 100;
        Self { structure, loaded: false, rate, clear_water: true, clear_lava: true, started: false }
    }

    pub fn show(&mut self, ctx: &Context, snapshot: &LibrarySnapshot, out: &mut Vec<LibraryAction>) -> DialogOutcome {
        let row = snapshot.build.as_ref().filter(|row| row.structure == self.structure);
        if self.ended(snapshot, row) {
            return DialogOutcome::Closed;
        }
        if let (false, Some(row)) = (self.loaded, row) {
            self.load(row.settings);
        }
        let Some(structure) = Self::structure(snapshot, self.structure) else {
            return DialogOutcome::Closed;
        };
        let total = row.map_or(0, |row| row.total);
        let blocks = Args::new()
            .user("name", UserText::from_model(structure.name.clone()))
            .uint("x", u64::from(structure.size.x))
            .uint("y", u64::from(structure.size.y))
            .uint("z", u64::from(structure.size.z))
            .user("blocks", UserText::from_model(DigitGroups::of(total)));
        let subtitle = Text::plural(StratumMessage::BuildSubtitle, total, blocks);
        let before = self.settings();
        let form =
            DialogForm::new(Text::message(StratumMessage::BuildTitle), Text::message(StratumMessage::BuildStart));
        let answer = form.subtitle(subtitle).blocked(Self::blocked(row)).show(ctx, |ui| self.body(ui, row));
        let after = self.settings();
        if self.loaded && after != before {
            out.push(LibraryAction::Build(BuildAction::SetSettings(self.structure, after)));
        }
        match answer {
            FormAnswer::Pending => DialogOutcome::Open,
            FormAnswer::Cancel => {
                out.push(LibraryAction::Build(BuildAction::Cancel));
                DialogOutcome::Closed
            }
            FormAnswer::Confirm => {
                if Self::blocked(row).is_none() {
                    self.started = true;
                    out.push(LibraryAction::Build(BuildAction::Start));
                }
                DialogOutcome::Open
            }
        }
    }

    // Takes the values the build has saved for the structure.
    fn load(&mut self, settings: BuildSettings) {
        self.loaded = true;
        self.rate = u32::from(settings.blocks_per_second) * 100;
        self.clear_water = settings.clear_water;
        self.clear_lava = settings.clear_lava;
    }

    // The values the panel shows, with the rate whole and inside the allowed range.
    fn settings(&self) -> BuildSettings {
        let low = f64::from(BuildSettings::MIN_RATE);
        let high = f64::from(BuildSettings::MAX_RATE);
        let rate = (f64::from(self.rate) / Self::HUNDREDTHS).round().clamp(low, high);
        BuildSettings { blocks_per_second: rate as u8, clear_water: self.clear_water, clear_lava: self.clear_lava }
    }

    // True when the panel has nothing left to do: the build runs, or it ended after Start, or it cannot start.
    fn ended(&self, snapshot: &LibrarySnapshot, row: Option<&BuildRow>) -> bool {
        let waiting = row.is_none_or(|row| matches!(row.phase, BuildPhase::Probing | BuildPhase::Ready));
        if waiting {
            return snapshot.build_allowed != Some(self.structure);
        }
        // A finished build of an earlier panel stays in the snapshot until this panel's own row replaces it.
        row.is_some_and(|row| row.phase.is_running()) || self.started
    }

    // The reason Start cannot be used now, or None when it can.
    fn blocked(row: Option<&BuildRow>) -> Option<Text> {
        match row {
            Some(row) if row.phase == BuildPhase::Ready && row.total > 0 => None,
            Some(row) if row.phase == BuildPhase::Ready => Some(Text::message(StratumMessage::BuildNothingToPlace)),
            _ => Some(Text::message(StratumMessage::BuildProbing)),
        }
    }

    fn structure(snapshot: &LibrarySnapshot, id: StructureId) -> Option<&StructureRow> {
        let projects = snapshot.projects.iter().flat_map(|project| project.structures.iter());
        projects.chain(snapshot.unsorted.iter()).find(|row| row.id == id)
    }

    fn body(&mut self, ui: &mut Ui, row: Option<&BuildRow>) {
        self.rate_row(ui);
        self.estimate_row(ui, row);
        self.clear_rows(ui);
        if let Some(notice) = row.and_then(|row| row.notice) {
            Self::notice(ui, notice);
        }
        if let Some(row) = row.filter(|row| !row.unplaceable.is_empty()) {
            Self::unplaceable(ui, row);
        }
    }

    // "Blocks per second", the rate field with its unit, and the allowed range.
    fn rate_row(&mut self, ui: &mut Ui) {
        let height = <spacing::Field as SpacingToken>::VALUE;
        let size = vec2(ui.available_width(), height);
        ui.allocate_ui_with_layout(size, Layout::left_to_right(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = <spacing::L as SpacingToken>::VALUE;
            Self::label(ui, StratumMessage::BuildRate, height);
            let low = BuildSettings::MIN_RATE;
            let high = BuildSettings::MAX_RATE;
            let range = Args::new().uint("min", u64::from(low)).uint("max", u64::from(high));
            let reason = Text::format(StratumMessage::BuildRateOutOfRange, range.clone());
            let mut rate = f64::from(self.rate) / Self::HUNDREDTHS;
            DragField::new(Id::unique(StratumId::DialogField), &mut rate)
                .unit(DragUnit::Text(Text::message(StratumMessage::BuildRateUnit)))
                .decimals(0)
                .step(Self::RATE_STEP)
                .range(f64::from(low)..=f64::from(high), reason)
                .show(ui, Self::FIELD_WIDTH);
            self.rate = (rate * Self::HUNDREDTHS).round().max(0.0) as u32;
            Caption::new(Text::format(StratumMessage::BuildRateRange, range)).show(ui);
        });
    }

    // "Estimated time": the blocks to place divided by the rate, rounded up.
    fn estimate_row(&self, ui: &mut Ui, row: Option<&BuildRow>) {
        let height = <spacing::Row as SpacingToken>::VALUE;
        let size = vec2(ui.available_width(), height);
        ui.allocate_ui_with_layout(size, Layout::left_to_right(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = <spacing::L as SpacingToken>::VALUE;
            Self::label(ui, StratumMessage::BuildEstimate, height);
            let rate = u64::from(self.settings().blocks_per_second);
            let seconds = BuildTime::seconds(row.map_or(0, |row| row.total), rate);
            let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
            let text = <color::Text as ColorToken>::VALUE;
            let at = pos2(rect.left(), rect.center().y);
            let time = BuildTime::estimate(seconds);
            Draw::text(ui.painter(), at, Align2::LEFT_CENTER, &time, Self::VALUE, text, TextFit::Line);
        });
    }

    // The label column of a setting row.
    fn label(ui: &mut Ui, label: StratumMessage, height: f32) {
        let (rect, _) = ui.allocate_exact_size(vec2(Self::LABEL_WIDTH, height), Sense::hover());
        let dim = <color::TextDim as ColorToken>::VALUE;
        let at = pos2(rect.left(), rect.center().y);
        let fit = TextFit::Truncate(Self::LABEL_WIDTH);
        Draw::text(ui.painter(), at, Align2::LEFT_CENTER, &Text::message(label), Self::LABEL, dim, fit);
    }

    // "Clear water" and "Clear lava".
    fn clear_rows(&mut self, ui: &mut Ui) {
        let on = |value: bool| if value { Selection::On } else { Selection::Off };
        let mut water = on(self.clear_water);
        let mut lava = on(self.clear_lava);
        Checkbox::new(&mut water).label(Text::message(StratumMessage::BuildClearWater)).show(ui);
        Checkbox::new(&mut lava).label(Text::message(StratumMessage::BuildClearLava)).show(ui);
        self.clear_water = water == Selection::On;
        self.clear_lava = lava == Selection::On;
    }

    // A warning line: an icon and one message, on the warning fill.
    fn notice(ui: &mut Ui, notice: BuildNotice) {
        let message = match notice {
            BuildNotice::StandNear => StratumMessage::BuildKeepFluidsFar,
            BuildNotice::TooManyKinds => StratumMessage::BuildTooManyKinds,
        };
        let pad_x = <spacing::M as SpacingToken>::VALUE;
        let pad_y = <spacing::S as SpacingToken>::VALUE;
        let note = TypeSize::Note.line_height();
        let icon = IconSize::Regular.points();
        let height = note.max(icon) + 2.0 * pad_y;
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        let painter = ui.painter();
        Draw::fill(painter, rect, Corner::Control, <color::WarningSoft as ColorToken>::VALUE);
        let slot = Rect::from_center_size(pos2(rect.left() + pad_x + icon / 2.0, rect.center().y), vec2(icon, icon));
        Icon::Warning.paint(painter, slot, IconSize::Regular, <color::Warning as ColorToken>::VALUE);
        let left = slot.right() + Self::NOTICE_GAP;
        let fit = TextFit::Truncate((rect.right() - pad_x - left).max(0.0));
        let at = pos2(left, rect.center().y);
        let text = <color::Text as ColorToken>::VALUE;
        Draw::text(painter, at, Align2::LEFT_CENTER, &Text::message(message), Self::VALUE, text, fit);
    }

    // The heading and the list of the blocks the game cannot place, one name a row.
    fn unplaceable(ui: &mut Ui, row: &BuildRow) {
        SectionHeader::new(Text::message(StratumMessage::BuildUnplaceable)).show(ui);
        let line = <spacing::Row as SpacingToken>::VALUE;
        let pad_y = <spacing::Xs as SpacingToken>::VALUE;
        let pad_x = <spacing::M as SpacingToken>::VALUE;
        let height = row.unplaceable.len() as f32 * line + 2.0 * pad_y;
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        let painter = ui.painter();
        Draw::fill(painter, rect, Corner::Control, <color::BgField as ColorToken>::VALUE);
        let text = <color::Text as ColorToken>::VALUE;
        for (index, name) in row.unplaceable.iter().enumerate() {
            let centre = rect.top() + pad_y + (index as f32 + 0.5) * line;
            let at = pos2(rect.left() + pad_x, centre);
            let name = Text::user(UserText::from_model(name.clone()));
            let fit = TextFit::Truncate((rect.width() - 2.0 * pad_x).max(0.0));
            Draw::text(painter, at, Align2::LEFT_CENTER, &name, Self::VALUE, text, fit);
        }
    }
}
