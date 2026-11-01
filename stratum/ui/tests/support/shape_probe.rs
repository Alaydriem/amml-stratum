use egui::epaint::ClippedShape;
use egui::{Color32, Pos2, Rect, Shape};

// Finds painted text in a frame's shapes, so a test can click where the user sees a label.
pub struct ShapeProbe;

impl ShapeProbe {
    // The rect of the first text that contains `needle`.
    pub fn text_rect(shapes: &[ClippedShape], needle: &str) -> Option<Rect> {
        shapes.iter().find_map(|clipped| Self::find(&clipped.shape, needle))
    }

    // The rects of every text that is exactly `needle`, in painted order.
    pub fn exact_rects(shapes: &[ClippedShape], needle: &str) -> Vec<Rect> {
        shapes.iter().flat_map(|clipped| Self::collect(&clipped.shape, needle)).collect()
    }

    // The rects of every rectangle filled with exactly `fill`, in painted order.
    pub fn fill_rects(shapes: &[ClippedShape], fill: Color32) -> Vec<Rect> {
        shapes.iter().flat_map(|clipped| Self::collect_fill(&clipped.shape, fill)).collect()
    }

    // How many painted shapes lie wholly inside `area`.
    pub fn count_inside(shapes: &[ClippedShape], area: Rect) -> usize {
        shapes.iter().map(|clipped| Self::inside(&clipped.shape, area)).sum()
    }

    fn inside(shape: &Shape, area: Rect) -> usize {
        match shape {
            Shape::Vec(shapes) => shapes.iter().map(|inner| Self::inside(inner, area)).sum(),
            Shape::Noop => 0,
            other => usize::from(area.contains_rect(other.visual_bounding_rect())),
        }
    }

    fn collect_fill(shape: &Shape, fill: Color32) -> Vec<Rect> {
        match shape {
            Shape::Rect(rect) if rect.fill == fill => vec![rect.rect],
            Shape::Vec(shapes) => shapes.iter().flat_map(|inner| Self::collect_fill(inner, fill)).collect(),
            _ => Vec::new(),
        }
    }

    fn collect(shape: &Shape, needle: &str) -> Vec<Rect> {
        match shape {
            Shape::Text(text) if text.galley.text() == needle => {
                vec![text.galley.rect.translate(text.pos.to_vec2())]
            }
            Shape::Vec(shapes) => shapes.iter().flat_map(|inner| Self::collect(inner, needle)).collect(),
            _ => Vec::new(),
        }
    }

    // The centre of the first text that contains `needle`.
    pub fn text_centre(shapes: &[ClippedShape], needle: &str) -> Option<Pos2> {
        Self::text_rect(shapes, needle).map(|rect| rect.center())
    }

    // The rect of the last text that contains `needle`: the one painted on top.
    pub fn last_text_rect(shapes: &[ClippedShape], needle: &str) -> Option<Rect> {
        shapes.iter().rev().find_map(|clipped| Self::find_last(&clipped.shape, needle))
    }

    // The centre of the last text that contains `needle`.
    pub fn last_text_centre(shapes: &[ClippedShape], needle: &str) -> Option<Pos2> {
        Self::last_text_rect(shapes, needle).map(|rect| rect.center())
    }

    // The centre of the last text that is exactly `text`.
    pub fn last_exact_centre(shapes: &[ClippedShape], text: &str) -> Option<Pos2> {
        shapes.iter().rev().find_map(|clipped| Self::find_exact(&clipped.shape, text)).map(|rect| rect.center())
    }

    fn find_exact(shape: &Shape, needle: &str) -> Option<Rect> {
        match shape {
            Shape::Text(text) if text.galley.text() == needle => Some(text.galley.rect.translate(text.pos.to_vec2())),
            Shape::Vec(shapes) => shapes.iter().rev().find_map(|inner| Self::find_exact(inner, needle)),
            _ => None,
        }
    }

    fn find_last(shape: &Shape, needle: &str) -> Option<Rect> {
        match shape {
            Shape::Text(text) if text.galley.text().contains(needle) => {
                Some(text.galley.rect.translate(text.pos.to_vec2()))
            }
            Shape::Vec(shapes) => shapes.iter().rev().find_map(|inner| Self::find_last(inner, needle)),
            _ => None,
        }
    }

    fn find(shape: &Shape, needle: &str) -> Option<Rect> {
        match shape {
            Shape::Text(text) if text.galley.text().contains(needle) => {
                Some(text.galley.rect.translate(text.pos.to_vec2()))
            }
            Shape::Vec(shapes) => shapes.iter().find_map(|inner| Self::find(inner, needle)),
            _ => None,
        }
    }
}
