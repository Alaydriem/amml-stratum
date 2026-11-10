use amml_ui::tokens::{ColorToken, SpacingToken, color, spacing};
use amml_ui::widget::{Chip, Toggle};
use egui::{Event, Pos2};

use super::{ShapeProbe, ViewSession};

// Where a test clicks in a sidebar marker row, measured from the painted panel.
pub struct RowSpots;

impl RowSpots {
    // The action button's side and the row padding before it.
    const SIDE: f32 = <spacing::Row as SpacingToken>::VALUE;
    const PAD: f32 = <spacing::M as SpacingToken>::VALUE;

    // The right edge of the row's content: Paper, Sidebar, 470 wide, 21 padding at the right; List row, 12 padding.
    fn right(session: &ViewSession) -> f32 {
        session.panel.min.x + 470.0 - 21.0 - Self::PAD
    }

    // The height of the row's centre, from its name label.
    fn centre(session: &ViewSession, name: &str) -> f32 {
        let label = ShapeProbe::text_centre(&session.shapes, name).expect("marker row is painted");
        // the name is the first line of the row's text block; the chip line under it is a 6 gap and a chip high
        label.y + (6.0 + Chip::height()) / 2.0
    }

    // The centre of the row's delete button.
    pub fn delete(session: &ViewSession, name: &str) -> Pos2 {
        Pos2::new(Self::right(session) - Self::SIDE / 2.0, Self::centre(session, name))
    }

    // The centre of the trailing action of a structure row, found from the row's hover fill. `None` when no row of that
    // name paints a hover fill under the pointer.
    pub fn structure_action(session: &mut ViewSession, name: &str) -> Option<Pos2> {
        let label = ShapeProbe::text_centre(&session.shapes, name)?;
        session.frame(vec![Event::PointerMoved(label)]);
        let hover = <color::BgHover as ColorToken>::VALUE;
        let row = ShapeProbe::fill_rects(&session.shapes, hover).into_iter().find(|rect| rect.contains(label))?;
        Some(Pos2::new(row.right() - Self::PAD - Self::SIDE / 2.0, row.center().y))
    }

    // The centre of the row's switch, left of the delete button.
    pub fn toggle(session: &ViewSession, name: &str) -> Pos2 {
        let right = Self::right(session) - Self::SIDE - Self::PAD;
        Pos2::new(right - Toggle::TRACK.x / 2.0, Self::centre(session, name))
    }
}
