use amml_ui::font::FontSource;
use amml_ui::theme::Theme;
use amml_ui::tokens::{ColorToken, color};
use egui::epaint::ClippedShape;
use egui::{Color32, Context, Pos2, RawInput, Rect, Shape, Ui, vec2};
use stratum_core::structs::structure::StructureSize;
use stratum_ui::views::SizeBox;

use crate::support::ShapeProbe;

// What `SizeBox::show` paints for one size: the field box, the faces and the labels.
struct Painted {
    shapes: Vec<ClippedShape>,
}

impl Painted {
    fn of(size: StructureSize) -> Self {
        let ctx = Context::default();
        Theme::install(&ctx, FontSource::BuiltIn);
        let mut shapes = Vec::new();
        for _ in 0..3 {
            let input = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(600.0, 400.0))),
                ..RawInput::default()
            };
            let mut output = ctx.run_ui(input, |ui: &mut Ui| SizeBox::show(ui, size));
            output.textures_delta.clear();
            shapes = output.shapes;
        }
        Self { shapes }
    }

    fn paths(shape: &Shape, out: &mut Vec<(Vec<Pos2>, Color32)>) {
        match shape {
            Shape::Path(path) => out.push((path.points.clone(), path.fill)),
            Shape::LineSegment { points, .. } => out.push((points.to_vec(), Color32::TRANSPARENT)),
            Shape::Vec(shapes) => shapes.iter().for_each(|inner| Self::paths(inner, out)),
            _ => {}
        }
    }

    fn all_paths(&self) -> Vec<(Vec<Pos2>, Color32)> {
        let mut out = Vec::new();
        self.shapes.iter().for_each(|clipped| Self::paths(&clipped.shape, &mut out));
        out
    }

    // The closed paths without a fill, in paint order: the left face, then the right face.
    fn outlines(&self) -> Vec<Vec<Pos2>> {
        self.all_paths()
            .into_iter()
            .filter(|(points, fill)| points.len() == 4 && *fill == Color32::TRANSPARENT)
            .map(|(points, _)| points)
            .collect()
    }

    fn filled(&self, fill: Color32) -> Vec<Pos2> {
        self.all_paths().into_iter().find(|(_, own)| *own == fill).map(|(points, _)| points).unwrap_or_default()
    }

    fn roof(&self) -> Vec<Pos2> {
        self.filled(<color::AccentSoft as ColorToken>::VALUE)
    }

    fn floor(&self) -> Vec<Pos2> {
        self.filled(<color::BgHover as ColorToken>::VALUE)
    }

    fn field(&self) -> Rect {
        ShapeProbe::fill_rects(&self.shapes, <color::BgField as ColorToken>::VALUE)[0]
    }

    // The lowest corner of the floor: the front corner of the block.
    fn front(&self) -> Pos2 {
        self.floor().into_iter().max_by(|a, b| a.y.total_cmp(&b.y)).expect("the floor is painted")
    }

    fn label(&self, text: &str) -> Rect {
        ShapeProbe::exact_rects(&self.shapes, text).first().copied().expect("the label is painted")
    }

    // Three faces with a side of at least `least` points each: the roof edges and the height of the front faces.
    fn faces_are_visible(&self, least: f32) -> bool {
        let roof = self.roof();
        let outlines = self.outlines();
        let left = outlines.first().cloned().unwrap_or_default();
        roof.len() == 4
            && left.len() == 4
            && (roof[0].distance(roof[1]) >= least)
            && (roof[1].distance(roof[2]) >= least)
            && (left[1].distance(left[2]) >= least)
    }
}

#[test]
fn a_flat_or_thin_structure_keeps_a_visible_face_on_every_side() {
    for size in [StructureSize::new(64, 1, 64), StructureSize::new(1, 200, 1), StructureSize::new(300, 2, 1)] {
        let painted = Painted::of(size);
        assert!(painted.faces_are_visible(4.0), "{size:?} loses a face");
    }
}

#[test]
fn a_structure_with_no_size_paints_finite_visible_faces() {
    let painted = Painted::of(StructureSize::new(0, 0, 0));
    assert!(painted.faces_are_visible(4.0));
    for (points, _) in painted.all_paths() {
        assert!(points.iter().all(|point| point.x.is_finite() && point.y.is_finite()));
    }
}

#[test]
fn the_box_keeps_the_proportions_of_the_structure() {
    let painted = Painted::of(StructureSize::new(32, 18, 24));
    let roof = painted.roof();
    let left = painted.outlines()[0].clone();
    // the roof's edge from the front corner to the right is the X side, the one to the left the Z side
    let x = roof[0].distance(roof[1]);
    let z = roof[0].distance(roof[3]);
    let y = left[1].distance(left[2]);
    assert!((x / z - 32.0 / 24.0).abs() < 0.01, "x {x}, z {z}");
    assert!((y / z - 18.0 / 24.0).abs() < 0.01, "y {y}, z {z}");
}

#[test]
fn the_block_stays_inside_the_field_box_for_any_size() {
    for size in [StructureSize::new(32, 18, 24), StructureSize::new(512, 4, 9), StructureSize::new(3, 384, 3)] {
        let painted = Painted::of(size);
        let field = painted.field();
        for (points, _) in painted.all_paths() {
            assert!(points.iter().all(|point| field.contains(*point)), "{size:?} leaves the box");
        }
    }
}

#[test]
fn the_axis_labels_sit_on_their_own_sides_of_the_block() {
    let painted = Painted::of(StructureSize::new(40, 11, 17));
    let front = painted.front();
    let right_edge = painted.outlines()[1].iter().map(|point| point.x).fold(f32::MIN, f32::max);
    assert!(painted.label("40").center().x > front.x, "X is right of the front corner");
    assert!(painted.label("17").center().x < front.x, "Z is left of the front corner");
    assert!(painted.label("11").min.x >= right_edge, "Y is right of the right vertical edge");
}

#[test]
fn a_wide_label_does_not_cross_the_edge_it_names() {
    let painted = Painted::of(StructureSize::new(120, 30, 100));
    let front = painted.front();
    let roof = painted.roof();
    let label = painted.label("120");
    // the bottom right edge runs from the front corner as far as the roof's front-to-right edge
    let edge = roof[1] - roof[0];
    for step in 0..=20 {
        let point = front + edge * (step as f32 / 20.0);
        assert!(!label.contains(point), "the label covers the edge at {point:?}");
    }
}

#[test]
fn a_wide_z_label_does_not_cross_its_edge() {
    let painted = Painted::of(StructureSize::new(100, 30, 120));
    let front = painted.front();
    let roof = painted.roof();
    let label = painted.label("120");
    let edge = roof[3] - roof[0];
    for step in 0..=20 {
        let point = front + edge * (step as f32 / 20.0);
        assert!(!label.contains(point), "the label covers the edge at {point:?}");
    }
}

#[test]
fn axis_labels_of_a_tall_thin_block_do_not_overlap() {
    let painted = Painted::of(StructureSize::new(3, 300, 4));
    assert!(!painted.label("3").intersects(painted.label("4")));
}
