use amml_i18n::text::{Text, UserText};
use amml_ui::draw::{Corner, Draw, TextFit, TextStyle, TypeSize};
use amml_ui::font::FontFace;
use amml_ui::tokens::{ColorToken, SpacingToken, color, spacing};
use egui::{Align2, Sense, Shape, Stroke, Ui, Vec2, vec2};
use stratum_core::structs::structure::StructureSize;

use crate::presenter::DigitGroups;

// The box of a structure's size: an isometric block drawn from its x, y and z lengths, each length labelled in its axis
// colour.
pub struct SizeBox;

impl SizeBox {
    /// Paper: Stratum, `Stratum · 01 Library`, node "Structure preview · size box": the field box's height, 152 px.
    const HEIGHT: f32 = 152.0;
    /// Paper: Stratum, `Stratum · 01 Library`, node "Structure preview · size box": the room the drawn block fills,
    /// measured from the sample's drawing in the box.
    const FIT: Vec2 = vec2(116.0, 110.0);
    /// Paper: Stratum, `Stratum · 01 Library`, node "Structure preview · size box": the sample outlines its faces with
    /// the `--spacing-stroke-thin` line. A side drawn four lines long keeps its face visible, so this is the shortest
    /// side drawn; the sample shows one size and sets no minimum.
    const MIN_SIDE: f32 = 4.0 * <spacing::StrokeThin as SpacingToken>::VALUE;
    /// Horizontal share of a unit along the X or Z axis in the isometric view, cos 30 degrees.
    const ISO_COS: f32 = 0.866_025_4;
    /// Vertical share of a unit along the X or Z axis in the isometric view, sin 30 degrees.
    const ISO_SIN: f32 = 0.5;
    /// Paper: Stratum, `Stratum · 01 Library`, node "Structure preview · size box": the opacity of the three hidden
    /// edges.
    const HIDDEN_OPACITY: f32 = 0.6;
    /// Paper: Stratum, `Stratum · 01 Library`, node "Structure preview · size box": the dash and the gap of the hidden
    /// edges.
    const DASH: (f32, f32) = (3.0, 4.0);
    /// Paper: Stratum, `Stratum · 01 Library`, node "Structure preview · size box": the X and Z labels' near corner,
    /// measured from the middle of the edge they name (the sample's labels sit 5 px right of and 2 px under it).
    const LABEL_NEAR: Vec2 = vec2(5.0, 2.0);
    /// Paper: Stratum, `Stratum · 01 Library`, node "Structure preview · size box": the room between the right vertical
    /// edge and the Y label, 8 px in the sample.
    const Y_GAP: f32 = 8.0;
    const LABEL: TextStyle = TextStyle::new(TypeSize::Caption, FontFace::Semibold);

    // The drawn lengths along X, Y and Z in points: the structure's proportions, scaled to fit `FIT`, none shorter
    // than `MIN_SIDE`.
    fn sides(size: StructureSize) -> [f32; 3] {
        let lengths = [size.x.max(1) as f32, size.y.max(1) as f32, size.z.max(1) as f32];
        let flat = lengths[0] + lengths[2];
        let across = Self::FIT.x / (flat * Self::ISO_COS);
        let high = Self::FIT.y / (flat * Self::ISO_SIN + lengths[1]);
        let scale = across.min(high);
        let drawn = lengths.map(|length| (length * scale).max(Self::MIN_SIDE));
        let width = (drawn[0] + drawn[2]) * Self::ISO_COS;
        let height = (drawn[0] + drawn[2]) * Self::ISO_SIN + drawn[1];
        let shrink = (Self::FIT.x / width).min(Self::FIT.y / height).min(1.0);
        drawn.map(|length| (length * shrink).max(Self::MIN_SIDE))
    }

    pub fn show(ui: &mut Ui, size: StructureSize) {
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), Self::HEIGHT), Sense::hover());
        let painter = ui.painter();
        Draw::fill(painter, rect, Corner::Control, <color::BgField as ColorToken>::VALUE);
        Draw::stroke(painter, rect, Corner::Control, <color::Line as ColorToken>::VALUE);

        let sides = Self::sides(size);
        let along_x = vec2(sides[0] * Self::ISO_COS, -sides[0] * Self::ISO_SIN);
        let along_z = vec2(-sides[2] * Self::ISO_COS, -sides[2] * Self::ISO_SIN);
        let up = vec2(0.0, -sides[1]);
        let top = along_x.y + along_z.y + up.y;
        // the front bottom corner, placed so the drawing is centred in the box
        let front = rect.center() - vec2((along_x.x + along_z.x) / 2.0, top / 2.0);
        let right = front + along_x;
        let left = front + along_z;
        let back = right + along_z;

        let thin = <spacing::StrokeThin as SpacingToken>::VALUE;
        let accent = Stroke::new(thin, <color::Accent as ColorToken>::VALUE);
        let faint = Stroke::new(thin, <color::TextFaint as ColorToken>::VALUE);
        let hidden = Stroke::new(thin, <color::TextFaint as ColorToken>::VALUE.gamma_multiply(Self::HIDDEN_OPACITY));

        let floor = <color::BgHover as ColorToken>::VALUE;
        painter.add(Shape::convex_polygon(vec![front, right, back, left], floor, Stroke::NONE));
        let roof = <color::AccentSoft as ColorToken>::VALUE;
        painter.add(Shape::convex_polygon(vec![front + up, right + up, back + up, left + up], roof, accent));
        painter.add(Shape::closed_line(vec![left, front, front + up, left + up], faint));
        painter.add(Shape::closed_line(vec![front, right, right + up, front + up], faint));
        for (from, to) in [(back, right), (back, left), (back, back + up)] {
            painter.extend(Shape::dashed_line(&[from, to], hidden, Self::DASH.0, Self::DASH.1));
        }

        // each label hangs from its near corner, so a long number grows away from the edge it names
        let near = Self::LABEL_NEAR;
        let x_at = front + along_x / 2.0 + near;
        let z_at = front + along_z / 2.0 + vec2(-near.x, near.y);
        let y_at = right + up / 2.0 + vec2(Self::Y_GAP, 0.0);
        let number = |value: u32| Text::user(UserText::from_model(DigitGroups::of(u64::from(value))));
        let axes = [
            (x_at, Align2::LEFT_TOP, size.x, <color::AxisX as ColorToken>::VALUE),
            (y_at, Align2::LEFT_CENTER, size.y, <color::AxisY as ColorToken>::VALUE),
            (z_at, Align2::RIGHT_TOP, size.z, <color::AxisZ as ColorToken>::VALUE),
        ];
        for (at, anchor, value, colour) in axes {
            Draw::text(painter, at, anchor, &number(value), Self::LABEL, colour, TextFit::Line);
        }
    }
}
