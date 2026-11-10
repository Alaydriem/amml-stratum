use std::sync::Arc;

use amm_client::link::{FrameHandler, FrameInputView, FrameReply};
use amm_core::structs::frame::LayerKind;
use egui::Context;

use super::{HudFrame, LibraryFrame, SharedUi};

// The one frame handler the mod registers. The manager calls it once per layer the mod holds: the Overlay pass
// goes to the library, the Hud pass to the HUD card.
pub struct FrameRouter {
    library: LibraryFrame,
    hud: HudFrame,
}

impl FrameRouter {
    pub fn new(shared: Arc<SharedUi>) -> Self {
        Self { library: LibraryFrame::new(Arc::clone(&shared)), hud: HudFrame::new(shared) }
    }
}

impl FrameHandler for FrameRouter {
    fn frame(&mut self, ctx: &Context, input: &FrameInputView) -> FrameReply {
        match input.layer() {
            LayerKind::Overlay => self.library.frame(ctx, input),
            LayerKind::Hud => self.hud.frame(ctx, input),
        }
    }
}
