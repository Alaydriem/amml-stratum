use std::sync::Arc;
use std::time::Instant;

use amm_client::link::{FrameHandler, FrameInputView, FrameReply};
use amm_core::structs::frame::{CursorShape, LayerKind};
use egui::{Context, Key, Modifiers, Rect};
use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::snapshot::LibrarySnapshot;
use stratum_ui::state::LibraryUiState;
use stratum_ui::views::LibraryView;

use super::SharedUi;

// The library's egui pass, on the render thread. It draws the last published snapshot on the Overlay layer
// while the library is open and sends the UI's actions to the tick. It never touches the controller.
// Each fresh open starts with a new view and new UI state, because the view's row cache is keyed by snapshot
// revision only. The pick follower is carried over, so a pick is acted on once.
pub struct LibraryFrame {
    shared: Arc<SharedUi>,
    view: LibraryView,
    state: LibraryUiState,
    snapshot: Arc<LibrarySnapshot>,
    // Filled by the view each frame and emptied into the shared queue; its capacity is kept.
    actions: Vec<LibraryAction>,
    // The open the view and state belong to.
    opens: u64,
}

impl LibraryFrame {
    pub fn new(shared: Arc<SharedUi>) -> Self {
        Self {
            view: Self::new_view(&shared),
            shared,
            state: LibraryUiState::new(),
            snapshot: Arc::new(LibrarySnapshot::empty()),
            actions: Vec::with_capacity(SharedUi::ACTION_CAPACITY),
            opens: 0,
        }
    }

    // The UI state the last pass drew with.
    pub fn state(&self) -> &LibraryUiState {
        &self.state
    }

    fn new_view(shared: &SharedUi) -> LibraryView {
        LibraryView::new(shared.preview().cloned())
    }

    fn nothing() -> FrameReply {
        FrameReply { cursor: CursorShape::Default, layer_rect: Rect::NOTHING }
    }

    fn draw(&mut self, ctx: &Context) -> FrameReply {
        let opens = self.shared.opens();
        if opens != self.opens {
            self.opens = opens;
            self.view = Self::new_view(&self.shared);
            // a pick the player already saw does not move the page or flash again
            let pick_flash = std::mem::take(&mut self.state.pick_flash).after_close();
            self.state = LibraryUiState::new();
            self.state.pick_flash = pick_flash;
        }
        if let Some(snapshot) = self.shared.try_snapshot() {
            self.snapshot = snapshot;
        }
        // a focused text field or an open dialog takes Escape for itself
        let escape_free = self.state.dialog.is_none() && !ctx.egui_wants_keyboard_input();
        let rect = self.view.show(ctx, &self.snapshot, &mut self.state, &mut self.actions);
        let escape = escape_free
            && self.state.dialog.is_none()
            && ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape));
        self.shared.send(&mut self.actions);
        if std::mem::take(&mut self.state.close_requested) || escape {
            self.shared.request_close();
        }
        FrameReply { cursor: CursorShape::Arrow, layer_rect: rect }
    }
}

impl FrameHandler for LibraryFrame {
    fn frame(&mut self, ctx: &Context, input: &FrameInputView) -> FrameReply {
        if input.layer() != LayerKind::Overlay || !self.shared.is_shown() {
            return Self::nothing();
        }
        if !self.shared.meter().enabled() {
            return self.draw(ctx);
        }
        let started = Instant::now();
        let reply = self.draw(ctx);
        self.shared.meter().record(u64::try_from(started.elapsed().as_micros()).unwrap_or(u64::MAX));
        reply
    }
}
