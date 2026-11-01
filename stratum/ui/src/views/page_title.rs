use amml_i18n::text::Text;
use amml_ui::draw::{Draw, TextFit, TextStyle, TypeSize};
use amml_ui::font::FontFace;
use amml_ui::text::UiText;
use amml_ui::tokens::{ColorToken, SpacingToken, color, spacing};
use amml_ui::widget::{IconButtonSize, MetaSeparator};
use egui::{Align, Align2, Layout, Rect, Response, Sense, Ui, UiBuilder, pos2, vec2};

// The title block of a detail page: the name, the facts under it joined by dots, and parts at the right.
pub struct PageTitle {
    title: Text,
    meta: Vec<Text>,
}

impl PageTitle {
    /// Paper: Stratum, `Stratum · 04 Project materials`, node "Detail": the gap between its parts.
    pub const SECTION_GAP: f32 = 24.0;
    /// Paper: Stratum, `Stratum · 18 Project menu`, node "Project header": the gap between the title and the part
    /// beside it, 10 px.
    const BESIDE_GAP: f32 = 10.0;
    const TITLE: TextStyle = TextStyle::new(TypeSize::Title, FontFace::Semibold);
    const META: TextStyle = TextStyle::new(TypeSize::Note, FontFace::Regular);

    pub fn new(title: Text) -> Self {
        Self { title, meta: Vec::new() }
    }

    pub fn meta(mut self, items: Vec<Text>) -> Self {
        self.meta = items;
        self
    }

    // Takes the full width. `trailing` is laid out right to left at the right edge.
    pub fn show(self, ui: &mut Ui, trailing: impl FnOnce(&mut Ui)) -> Response {
        self.layout(ui, None::<fn(&mut Ui)>, trailing)
    }

    // Like `show`, with `beside` laid out left to right straight after the title.
    pub fn show_beside(self, ui: &mut Ui, beside: impl FnOnce(&mut Ui), trailing: impl FnOnce(&mut Ui)) -> Response {
        self.layout(ui, Some(beside), trailing)
    }

    fn layout(self, ui: &mut Ui, beside: Option<impl FnOnce(&mut Ui)>, trailing: impl FnOnce(&mut Ui)) -> Response {
        let gap = <spacing::Xs as SpacingToken>::VALUE;
        let height = TypeSize::Title.line_height() + gap + TypeSize::Note.line_height();
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        let mut right = ui.new_child(UiBuilder::new().max_rect(rect).layout(Layout::right_to_left(Align::Center)));
        trailing(&mut right);
        let room = (right.min_rect().left() - rect.left() - <spacing::L as SpacingToken>::VALUE).max(0.0);
        let room = if right.min_rect().width() > 0.0 { room } else { rect.width() };

        // the part beside the title keeps its room, so a long title is cut short of it
        let kept = if beside.is_some() { Self::BESIDE_GAP + IconButtonSize::Regular.side() } else { 0.0 };
        let text = <color::Text as ColorToken>::VALUE;
        let title = Draw::text(
            ui.painter(),
            rect.left_top(),
            Align2::LEFT_TOP,
            &self.title,
            Self::TITLE,
            text,
            TextFit::Truncate((room - kept).max(0.0)),
        );
        let after = Rect::from_min_max(
            pos2(title.right() + Self::BESIDE_GAP, rect.top()),
            pos2(rect.left() + room, rect.top() + TypeSize::Title.line_height()),
        );
        if let Some(beside) = beside {
            let layout = Layout::left_to_right(Align::Center);
            let mut beside_ui = ui.new_child(UiBuilder::new().max_rect(after).layout(layout));
            beside(&mut beside_ui);
        }
        let painter = ui.painter();
        let dim = <color::TextDim as ColorToken>::VALUE;
        let top = rect.top() + TypeSize::Title.line_height() + gap;
        let space = <spacing::S as SpacingToken>::VALUE;
        let mut x = rect.left();
        for (index, item) in self.meta.iter().enumerate() {
            if index > 0 {
                MetaSeparator::paint(
                    painter,
                    pos2(x + space + MetaSeparator::SIDE / 2.0, top + TypeSize::Note.line_height() / 2.0),
                );
                x += 2.0 * space + MetaSeparator::SIDE;
            }
            let width = Draw::measure(ui.ctx(), &UiText::resolve(ui.ctx(), item), Self::META).x;
            Draw::text(
                painter,
                pos2(x, top),
                Align2::LEFT_TOP,
                item,
                Self::META,
                dim,
                TextFit::Truncate((rect.left() + room - x).max(0.0)),
            );
            x += width;
        }
        response
    }
}
