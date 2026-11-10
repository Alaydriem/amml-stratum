use amml_i18n::text::{Args, Text, UserText};
use amml_ui::draw::{Draw, TextFit, TextStyle, TypeSize};
use amml_ui::font::FontFace;
use amml_ui::tokens::rule::Elevation;
use amml_ui::tokens::{ColorToken, SpacingToken, color, spacing};
use amml_ui::widget::{MetaSeparator, Surface, SurfaceLevel};
use egui::{Align2, Sense, Ui, pos2, vec2};
use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::marker::MarkerTarget;
use stratum_core::structs::snapshot::{LibrarySnapshot, StructureRow};

use super::{CardFooter, ItemActions, MarkerActions, SizeBox, StructurePreview};
use crate::message::StratumMessage;
use crate::presenter::DigitGroups;
use crate::state::{DialogItem, LibraryUiState};

// The card of one structure: the 3D preview (or the size box when there is no preview), one line of facts (size,
// layers, blocks and the saved origin), its actions, and a footer with the marker buttons pinned to the card's
// bottom.
pub struct FactsPanel;

impl FactsPanel {
    /// Paper: Stratum, `Stratum · 20 Library · 3D preview`, node "Facts panel": the gap between the preview and the
    /// facts line, its `gap` of 4 px (`--spacing-xs`) and the line's `padding-top` of 12 px (`--spacing-m`).
    const FACTS_GAP: f32 = <spacing::Xs as SpacingToken>::VALUE + <spacing::M as SpacingToken>::VALUE;
    /// Paper: Stratum, `Stratum · 20 Library · 3D preview`, node "Facts panel": the gap between the facts line and
    /// the actions, its `gap` of 4 px (`--spacing-xs`) and the actions' `padding-top` of 16 px (`--spacing-l`).
    const ACTIONS_GAP: f32 = <spacing::Xs as SpacingToken>::VALUE + <spacing::L as SpacingToken>::VALUE;
    /// Paper: Stratum, `Stratum · 20 Library · 3D preview`, node "Facts panel": its `padding-bottom`, 20 px, the
    /// room between the actions and the footer divider.
    const FOOTER_GAP: f32 = 20.0;
    /// Paper: Stratum, `Stratum · 20 Library · 3D preview`, node "Facts panel": its `min-height` of 360 px, less its
    /// padding (2 x 20), the facts line (30), the actions (52) and the gaps (2 x 4). The preview never gets lower.
    const PREVIEW_MIN_HEIGHT: f32 = 230.0;
    const FACT: TextStyle = TextStyle::new(TypeSize::Help, FontFace::Regular);

    // `height` is the room the card fills from where it starts, so the footer sits at the bottom of that room.
    // Returns the action of Place marker here when it was clicked.
    pub fn show(
        ui: &mut Ui,
        snapshot: &LibrarySnapshot,
        row: &StructureRow,
        state: &mut LibraryUiState,
        height: f32,
        preview: Option<&mut StructurePreview>,
    ) -> Option<LibraryAction> {
        let bottom = ui.cursor().top() + height;
        let frame = Surface::frame(SurfaceLevel::Raised, Elevation::Flat);
        let inset = CardFooter::inset(&frame);
        let mut placed = None;
        frame.show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 0.0;
            match preview {
                Some(preview) => {
                    let height = Self::preview_height(ui, bottom, inset);
                    preview.show(ui, row, height);
                }
                None => SizeBox::show(ui, row.size),
            }
            ui.add_space(Self::FACTS_GAP);
            Self::facts_line(ui, row);
            ui.add_space(Self::ACTIONS_GAP);
            ItemActions::show(ui, snapshot, DialogItem::Structure(row.id), Some(row), state);
            CardFooter::show(ui, bottom, inset, |ui| {
                placed = MarkerActions::show(ui, snapshot, MarkerTarget::Structure(row.id), None, state);
            });
        });
        placed
    }

    // The room the preview takes: what is left above the footer once the facts line and the actions have theirs.
    fn preview_height(ui: &Ui, card_bottom: f32, inset: f32) -> f32 {
        let below = Self::FACTS_GAP
            + TypeSize::Help.line_height()
            + Self::ACTIONS_GAP
            + <spacing::Button as SpacingToken>::VALUE
            + Self::FOOTER_GAP;
        let free = CardFooter::divider(card_bottom, inset) - ui.cursor().top() - below;
        free.max(Self::PREVIEW_MIN_HEIGHT)
    }

    // Size, layers, blocks and the saved origin on one line, each pair split by a drawn dot.
    fn facts_line(ui: &mut Ui, row: &StructureRow) {
        let line = TypeSize::Help.line_height();
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), line), Sense::hover());
        let size = Args::new()
            .uint("x", u64::from(row.size.x))
            .uint("y", u64::from(row.size.y))
            .uint("z", u64::from(row.size.z));
        let layers = u64::from(row.size.y);
        let blocks = Args::new().user("blocks", UserText::from_model(DigitGroups::of(row.block_count)));
        let origin = match row.origin {
            Some([x, y, z]) => Text::format(
                StratumMessage::FactOrigin,
                Args::new().int("x", i64::from(x)).int("y", i64::from(y)).int("z", i64::from(z)),
            ),
            None => Text::message(StratumMessage::FactNoOrigin),
        };
        let text = <color::Text as ColorToken>::VALUE;
        let facts = [
            (Text::format(StratumMessage::SizeDims, size), text),
            (Text::plural(StratumMessage::LayerCount, layers, Args::new()), text),
            (Text::plural(StratumMessage::BlocksCount, row.block_count, blocks), text),
            (origin, <color::TextDim as ColorToken>::VALUE),
        ];
        let gap = <spacing::S as SpacingToken>::VALUE;
        let painter = ui.painter();
        let centre = rect.center().y;
        let mut x = rect.left();
        for (index, (fact, colour)) in facts.iter().enumerate() {
            if index > 0 {
                MetaSeparator::paint(painter, pos2(x + gap + MetaSeparator::SIDE / 2.0, centre));
                x += 2.0 * gap + MetaSeparator::SIDE;
            }
            let room = (rect.right() - x).max(0.0);
            let at = pos2(x, centre);
            x = Draw::text(painter, at, Align2::LEFT_CENTER, fact, Self::FACT, *colour, TextFit::Truncate(room))
                .right();
        }
    }
}
