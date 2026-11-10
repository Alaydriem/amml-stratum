use amml_i18n::text::Text;
use amml_ui::state::DisabledReason;
use amml_ui::tokens::{SpacingToken, spacing};
use amml_ui::widget::{ButtonKind, ModalFrame, TextButton};
use egui::{Context, Rect, Ui, pos2, vec2};

use super::FormAnswer;
use crate::message::StratumMessage;
use crate::views::StratumId;

// The frame every dialog shares: a modal card with the dialog's body, then the confirm and Cancel buttons.
pub struct DialogForm {
    title: Text,
    confirm: Text,
    kind: ButtonKind,
    blocked: Option<Text>,
}

impl DialogForm {
    pub fn new(title: Text, confirm: Text) -> Self {
        Self { title, confirm, kind: ButtonKind::Primary, blocked: None }
    }

    // The confirm button is the danger button.
    pub fn danger(mut self) -> Self {
        self.kind = ButtonKind::Danger;
        self
    }

    // The reason the confirm button cannot be used now, if there is one.
    pub fn blocked(mut self, reason: Option<Text>) -> Self {
        self.blocked = reason;
        self
    }

    pub fn show(self, ctx: &Context, body: impl FnOnce(&mut Ui)) -> FormAnswer {
        let mut answer = FormAnswer::Pending;
        let Self { title, confirm, kind, blocked } = self;
        let modal = ModalFrame::new(egui::Id::unique(StratumId::Dialog), title);
        let response = modal.show(ctx, body, |ui| {
            let mut button = TextButton::new(kind, confirm);
            if let Some(reason) = blocked {
                button = button.enabled(false, DisabledReason::new(reason));
            }
            if button.show(ui).clicked() {
                answer = FormAnswer::Confirm;
            }
            if TextButton::new(ButtonKind::Secondary, Text::message(StratumMessage::Cancel)).show(ui).clicked() {
                answer = FormAnswer::Cancel;
            }
        });
        if response.closed && Self::dismissed(ctx, response.card) { FormAnswer::Cancel } else { answer }
    }

    // The modal frame reports a close for any click that lands on a part of the card that takes no click, such as a
    // disabled button. A close with no click is Escape, which the modal consumes. A close with a click counts only
    // for a click outside the card or on the close button.
    fn dismissed(ctx: &Context, card: Rect) -> bool {
        let click = ctx.input(|input| input.pointer.any_click().then(|| input.pointer.interact_pos()).flatten());
        click.is_none_or(|pos| !card.contains(pos) || Self::close_button(card).contains(pos))
    }

    // The close button at the top right of the card header.
    fn close_button(card: Rect) -> Rect {
        let edge = <spacing::Border as SpacingToken>::VALUE + <spacing::L as SpacingToken>::VALUE;
        let side = <spacing::Row as SpacingToken>::VALUE;
        Rect::from_min_size(pos2(card.right() - edge - side, card.top() + edge), vec2(side, side))
    }
}
