use std::sync::Arc;

use amm_client::link::{FrameHandler, FrameInputView, FrameReply};
use amm_core::structs::frame::{CursorShape, LayerKind};
use egui::{Context, Rect};
use stratum_core::structs::hud::HudState;
use stratum_core::structs::snapshot::LibrarySnapshot;
use stratum_ui::views::HudCard;

use super::SharedUi;

// The HUD card's egui pass, on the render thread. It draws the card row of the last published snapshot on the
// Hud layer while the tick publishes a HUD state, which it does only while the Hud lease is held. It reads the
// snapshot and the state and nothing else; the card row is built by the tick.
pub struct HudFrame {
    shared: Arc<SharedUi>,
    card: HudCard,
    snapshot: Arc<LibrarySnapshot>,
    // The state last read, kept for a frame that finds the lock held by the tick.
    state: Option<HudState>,
}

impl HudFrame {
    pub fn new(shared: Arc<SharedUi>) -> Self {
        Self { shared, card: HudCard::new(), snapshot: Arc::new(LibrarySnapshot::empty()), state: None }
    }

    fn nothing() -> FrameReply {
        FrameReply { cursor: CursorShape::Default, layer_rect: Rect::NOTHING }
    }
}

impl FrameHandler for HudFrame {
    fn frame(&mut self, ctx: &Context, input: &FrameInputView) -> FrameReply {
        if input.layer() != LayerKind::Hud {
            return Self::nothing();
        }
        if let Some(state) = self.shared.try_hud() {
            self.state = state;
        }
        let Some(state) = self.state else {
            return Self::nothing();
        };
        if let Some(snapshot) = self.shared.try_snapshot() {
            self.snapshot = snapshot;
        }
        let Some(row) = self.snapshot.hud.as_ref() else {
            return Self::nothing();
        };
        let rect = self.card.show(ctx, row, &state);
        FrameReply { cursor: CursorShape::Default, layer_rect: rect }
    }
}
