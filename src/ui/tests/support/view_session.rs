use amml_ui::font::FontSource;
use amml_ui::theme::Theme;
use egui::epaint::ClippedShape;
use egui::{
    Context, Event, Key, Modifiers, MouseWheelSource, MouseWheelUnit, OutputCommand, PointerButton, Pos2, RawInput,
    Rect, TouchPhase, Vec2, vec2,
};
use stratum_core::config::PreviewConfig;
use stratum_core::services::preview::PreviewService;
use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::preview::PreviewSource;
use stratum_core::structs::snapshot::LibrarySnapshot;
use stratum_ui::state::LibraryUiState;
use stratum_ui::views::LibraryView;

// One egui context that keeps a view, its state and the actions it pushed across frames, with synthetic pointer input.
pub struct ViewSession {
    ctx: Context,
    view: LibraryView,
    pub state: LibraryUiState,
    pub snapshot: LibrarySnapshot,
    pub actions: Vec<LibraryAction>,
    pub panel: Rect,
    pub shapes: Vec<ClippedShape>,
    // Every platform command the frames asked for, such as copying text to the clipboard.
    pub commands: Vec<OutputCommand>,
    time: f64,
    size: Vec2,
}

impl ViewSession {
    // Pointer steps between the press and the release of a drag.
    const DRAG_STEPS: usize = 4;

    pub fn screen() -> Rect {
        Rect::from_min_size(Pos2::ZERO, vec2(1920.0, 1080.0))
    }

    // A session that has drawn a few frames, so the layout has settled.
    pub fn new(snapshot: LibrarySnapshot) -> Self {
        Self::with_view(snapshot, LibraryView::new(None))
    }

    // Like `new`, with the 3D preview in the structure card. The service has no textures and no structure files,
    // so the pane shows that it has no picture.
    pub fn with_preview(snapshot: LibrarySnapshot) -> Self {
        Self::with_preview_files(snapshot, std::env::temp_dir().join("stratum-none"))
    }

    // Like `with_preview`, reading the structure files from `structures`. With no textures the blocks draw in the
    // grey stand-in.
    pub fn with_preview_files(snapshot: LibrarySnapshot, structures: std::path::PathBuf) -> Self {
        let service = PreviewService::new_shared(PreviewSource::new(None, structures), &PreviewConfig::default());
        Self::with_view(snapshot, LibraryView::new(Some(service)))
    }

    fn with_view(snapshot: LibrarySnapshot, view: LibraryView) -> Self {
        let ctx = Context::default();
        Theme::install(&ctx, FontSource::BuiltIn);
        let mut session = Self {
            ctx,
            view,
            state: LibraryUiState::new(),
            snapshot,
            actions: Vec::new(),
            panel: Rect::NOTHING,
            shapes: Vec::new(),
            commands: Vec::new(),
            time: 0.0,
            size: Self::screen().size(),
        };
        for _ in 0..3 {
            session.frame(Vec::new());
        }
        session
    }

    // Changes the screen size the next frames draw into.
    pub fn resize(&mut self, size: Vec2) {
        self.size = size;
    }

    // Scrolls the wheel by `delta` points over `pos`, then draws a few frames so the scroll settles.
    pub fn scroll(&mut self, pos: Pos2, delta: Vec2) {
        self.frame(vec![Event::PointerMoved(pos)]);
        self.frame(vec![Event::MouseWheel {
            unit: MouseWheelUnit::Point,
            delta,
            phase: TouchPhase::Move,
            source: MouseWheelSource::Unknown,
            modifiers: Modifiers::NONE,
        }]);
        for _ in 0..10 {
            self.frame(Vec::new());
        }
    }

    // Draws one frame with `events`. The shapes of the frame are kept in `shapes`.
    pub fn frame(&mut self, events: Vec<Event>) {
        self.time += 0.1;
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.size)),
            time: Some(self.time),
            events,
            ..RawInput::default()
        };
        let Self { ctx, view, state, snapshot, actions, panel, .. } = self;
        let mut output = ctx.run_ui(input, |ui| {
            *panel = view.show(ui.ctx(), snapshot, state, actions);
        });
        output.textures_delta.clear();
        self.commands.append(&mut output.platform_output.commands);
        self.shapes = output.shapes;
    }

    // Rests the pointer on `pos` for long enough that a tooltip opens.
    pub fn hover(&mut self, pos: Pos2) {
        self.frame(vec![Event::PointerMoved(pos)]);
        for _ in 0..12 {
            self.frame(Vec::new());
        }
    }

    // Moves the pointer to `pos`, presses and releases the primary button, each in its own frame.
    pub fn click(&mut self, pos: Pos2) {
        self.frame(vec![Event::PointerMoved(pos)]);
        self.frame(vec![Self::button(pos, true)]);
        self.frame(vec![Self::button(pos, false)]);
        // a settled frame, so a state change shows in `shapes`
        self.frame(Vec::new());
    }

    // Two presses at `pos` close enough in time for egui to read a double click.
    pub fn double_click(&mut self, pos: Pos2) {
        self.frame(vec![Event::PointerMoved(pos)]);
        for _ in 0..2 {
            self.frame(vec![Self::button(pos, true)]);
            self.frame(vec![Self::button(pos, false)]);
        }
        self.frame(Vec::new());
    }

    // Presses and releases `key`, each in its own frame.
    pub fn press(&mut self, key: Key) {
        self.frame(vec![Self::key_event(key, true)]);
        self.frame(vec![Self::key_event(key, false)]);
        self.frame(Vec::new());
    }

    // Types `text` into the field that has the keyboard.
    pub fn type_text(&mut self, text: &str) {
        self.frame(vec![Event::Text(text.to_owned())]);
        self.frame(Vec::new());
    }

    fn key_event(key: Key, pressed: bool) -> Event {
        Event::Key { key, physical_key: Some(key), pressed, repeat: false, modifiers: Modifiers::NONE }
    }

    // Presses at `from`, moves to `to` in a few frames and releases there, so egui sees a drag.
    pub fn drag(&mut self, from: Pos2, to: Pos2) {
        self.frame(vec![Event::PointerMoved(from)]);
        self.frame(vec![Self::button(from, true)]);
        for step in 1..=Self::DRAG_STEPS {
            let at = from + (to - from) * (step as f32 / Self::DRAG_STEPS as f32);
            self.frame(vec![Event::PointerMoved(at)]);
        }
        self.frame(vec![Self::button(to, false)]);
        self.frame(Vec::new());
    }

    fn button(pos: Pos2, pressed: bool) -> Event {
        Event::PointerButton { pos, button: PointerButton::Primary, pressed, modifiers: Modifiers::NONE }
    }
}
