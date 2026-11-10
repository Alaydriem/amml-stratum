use std::sync::Arc;

use amml_i18n::text::{Args, Text};
use amml_ui::draw::{Corner, Draw, TextFit, TextStyle, TypeSize};
use amml_ui::font::FontFace;
use amml_ui::text::UiText;
use amml_ui::tokens::{ColorToken, color};
use amml_ui::widget::MetaSeparator;
use egui::{Align2, Color32, ColorImage, Event, Rect, Sense, TextureHandle, TextureOptions, Ui, Vec2, pos2, vec2};
use stratum_core::services::preview::{PreviewRasterizer, PreviewService};
use stratum_core::structs::library::StructureId;
use stratum_core::structs::preview::{OrbitCamera, PreviewImage, PreviewRequest, PreviewSerials};
use stratum_core::structs::snapshot::StructureRow;

use crate::message::StratumMessage;

// The 3D preview of one structure: the picture the preview thread drew, turned by dragging and zoomed with the
// wheel in the pane. The last render time shows in a pill at the top right, the hint at the bottom left. A new
// picture is asked for only when the structure, the turn, the zoom or the pane size changes.
pub struct StructurePreview {
    service: Arc<PreviewService>,
    texture: Option<TextureHandle>,
    // The pixels last handed to the texture. egui lets go of its copy once the picture is uploaded; the next
    // picture is then written into it, so a steady drag allocates no new image.
    image: Option<Arc<ColorImage>>,
    // The structure the texture shows.
    shown: Option<StructureId>,
    camera: OrbitCamera,
    // The structure the camera turn belongs to.
    framed: Option<StructureId>,
    // The last request sent: structure, turn and image size.
    sent: Option<(StructureId, OrbitCamera, [u32; 2])>,
    serials: PreviewSerials,
    render_us: Option<u64>,
    failed: Option<StructureId>,
}

impl StructurePreview {
    /// Paper: Stratum, `Stratum · 20 Library · 3D preview`, node "Render time": `right` 10 px and `top` 10 px, the
    /// room between the pill and the pane's top right corner.
    const PILL_INSET: Vec2 = vec2(10.0, 10.0);
    /// Paper: Stratum, `Stratum · 20 Library · 3D preview`, node "Render time": its padding, 8 px at the sides and
    /// 2 px above and below.
    const PILL_PADDING: Vec2 = vec2(8.0, 2.0);
    /// Paper: Stratum, `Stratum · 20 Library · 3D preview`, node "Hint": `left` 12 px and `bottom` 10 px, the room
    /// between the hint and the pane's bottom left corner.
    const HINT_INSET: Vec2 = vec2(12.0, 10.0);
    /// Paper: Stratum, `Stratum · 20 Library · 3D preview`, node "Hint": the `gap` between its parts, 6 px.
    const HINT_GAP: f32 = 6.0;
    /// Degrees the camera turns per point of drag.
    const DEGREES_PER_POINT: f32 = 0.5;
    const TIME: TextStyle = TextStyle::new(TypeSize::Caption, FontFace::Regular);
    const NOTE: TextStyle = TextStyle::new(TypeSize::Label, FontFace::Regular);

    pub fn new(service: Arc<PreviewService>) -> Self {
        Self {
            service,
            texture: None,
            image: None,
            shown: None,
            camera: OrbitCamera::start(),
            framed: None,
            sent: None,
            serials: PreviewSerials::new(),
            render_us: None,
            failed: None,
        }
    }

    // The turn and zoom the pane shows now.
    pub fn camera(&self) -> OrbitCamera {
        self.camera
    }

    // Draws the pane `height` points high across the width it is given.
    pub fn show(&mut self, ui: &mut Ui, row: &StructureRow, height: f32) {
        let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::drag());
        if self.framed != Some(row.id) {
            self.framed = Some(row.id);
            self.camera = OrbitCamera::start();
            self.render_us = None;
        }
        if response.dragged() {
            let delta = response.drag_delta();
            if delta != Vec2::ZERO {
                self.camera = self.camera.turned(-delta.x * Self::DEGREES_PER_POINT, delta.y * Self::DEGREES_PER_POINT);
            }
        }
        if response.contains_pointer() {
            let steps = Self::take_wheel(ui);
            if steps != 0 {
                self.camera = self.camera.zoomed(steps);
            }
        }
        let size = Self::image_size(ui, rect);
        self.request(row, size);
        self.receive(ui, row.id);
        if self.serials.waiting() {
            ui.ctx().request_repaint();
        }
        self.paint(ui, rect, row.id);
    }

    // The picture size in pixels: the pane, capped at the rasteriser's largest picture with the aspect kept.
    fn image_size(ui: &Ui, rect: Rect) -> [u32; 2] {
        let pixels = rect.size() * ui.ctx().pixels_per_point();
        let scale = (PreviewRasterizer::MAX_WIDTH as f32 / pixels.x.max(1.0))
            .min(PreviewRasterizer::MAX_HEIGHT as f32 / pixels.y.max(1.0))
            .min(1.0);
        [(pixels.x * scale).round().max(1.0) as u32, (pixels.y * scale).round().max(1.0) as u32]
    }

    // The wheel steps of this frame, in when positive. The scroll is used up here, so the page behind the pane does
    // not move.
    fn take_wheel(ui: &mut Ui) -> i32 {
        ui.input_mut(|input| {
            let steps = input
                .events
                .iter()
                .map(|event| match event {
                    Event::MouseWheel { delta, .. } if delta.y > 0.0 => 1,
                    Event::MouseWheel { delta, .. } if delta.y < 0.0 => -1,
                    _ => 0,
                })
                .sum();
            input.smooth_scroll_delta = Vec2::ZERO;
            steps
        })
    }

    // Sends a request when the structure, the turn or the image size changed. A full queue is tried again on the
    // next frame.
    fn request(&mut self, row: &StructureRow, size: [u32; 2]) {
        let wanted = (row.id, self.camera, size);
        if self.sent == Some(wanted) {
            return;
        }
        let serial = self.serials.next();
        if self.service.submit(self.request_for(row, size, serial)).is_ok() {
            self.serials.sent(serial);
            self.sent = Some(wanted);
        }
    }

    fn request_for(&self, row: &StructureRow, size: [u32; 2], serial: u64) -> PreviewRequest {
        PreviewRequest {
            serial,
            structure: row.id,
            file_name: row.file_name.clone(),
            camera: self.camera,
            width: size[0],
            height: size[1],
        }
    }

    fn receive(&mut self, ui: &Ui, structure: StructureId) {
        let Some(frame) = self.service.take() else {
            return;
        };
        if !self.serials.answer(frame.serial) || frame.structure != structure {
            if let Ok(image) = frame.outcome {
                self.service.give_back(image.rgba);
            }
            return;
        }
        match frame.outcome {
            Ok(image) => {
                self.render_us = Some(image.stats.render_us);
                self.upload(ui, &image);
                self.service.give_back(image.rgba);
                self.shown = Some(structure);
                self.failed = None;
            }
            Err(_) => self.failed = Some(structure),
        }
    }

    fn upload(&mut self, ui: &Ui, image: &PreviewImage) {
        let size = [image.width as usize, image.height as usize];
        match self.image.as_mut().and_then(Arc::get_mut) {
            Some(pixels) if pixels.size == size => {
                for (out, texel) in pixels.pixels.iter_mut().zip(image.rgba.as_chunks::<4>().0) {
                    *out = Color32::from_rgba_unmultiplied(texel[0], texel[1], texel[2], texel[3]);
                }
            }
            _ => self.image = Some(Arc::new(ColorImage::from_rgba_unmultiplied(size, &image.rgba))),
        }
        let Some(pixels) = self.image.as_ref().map(Arc::clone) else {
            return;
        };
        match &mut self.texture {
            Some(texture) => texture.set(pixels, TextureOptions::LINEAR),
            None => self.texture = Some(ui.ctx().load_texture("stratum-preview", pixels, TextureOptions::LINEAR)),
        }
    }

    fn paint(&self, ui: &Ui, rect: Rect, structure: StructureId) {
        let texture = self.texture.as_ref().filter(|_| self.shown == Some(structure));
        let painter = &ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
        Draw::fill(painter, rect, Corner::Control, <color::BgField as ColorToken>::VALUE);
        Draw::stroke(painter, rect, Corner::Control, <color::Line as ColorToken>::VALUE);
        let faint = <color::TextFaint as ColorToken>::VALUE;
        match texture {
            Some(texture) => {
                let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
                painter.image(texture.id(), rect, uv, egui::Color32::WHITE);
            }
            None => {
                let note = if self.failed == Some(structure) {
                    StratumMessage::PreviewFailed
                } else {
                    StratumMessage::PreviewDrawing
                };
                let text = Text::message(note);
                Draw::text(painter, rect.center(), Align2::CENTER_CENTER, &text, Self::NOTE, faint, TextFit::Line);
            }
        }
        if let Some(render_us) = self.render_us {
            Self::paint_time(painter, rect, render_us);
        }
        Self::paint_hint(painter, rect);
    }

    // The render time in a pill at the top right.
    fn paint_time(painter: &egui::Painter, rect: Rect, render_us: u64) {
        let faint = <color::TextFaint as ColorToken>::VALUE;
        let args = Args::new().float("ms", render_us as f64 / 1000.0, 1);
        let text = Text::format(StratumMessage::PreviewRenderTime, args);
        let text = UiText::resolve(painter.ctx(), &text);
        let galley = Draw::galley(painter, &text, Self::TIME, faint, TextFit::Line);
        let size = galley.size() + Self::PILL_PADDING * 2.0;
        let top_right = pos2(rect.right() - Self::PILL_INSET.x, rect.top() + Self::PILL_INSET.y);
        let pill = Rect::from_min_size(pos2(top_right.x - size.x, top_right.y), size);
        Draw::fill(painter, pill, Corner::Small, <color::Pill as ColorToken>::VALUE);
        painter.galley(pill.min + Self::PILL_PADDING, galley, faint);
    }

    // "Drag to turn", a drawn dot, then "Scroll to zoom", at the bottom left.
    fn paint_hint(painter: &egui::Painter, rect: Rect) {
        let faint = <color::TextFaint as ColorToken>::VALUE;
        let centre = rect.bottom() - Self::HINT_INSET.y - TypeSize::Caption.line_height() / 2.0;
        let drag = Text::message(StratumMessage::PreviewHintDrag);
        let at = pos2(rect.left() + Self::HINT_INSET.x, centre);
        let drag = Draw::text(painter, at, Align2::LEFT_CENTER, &drag, Self::TIME, faint, TextFit::Line);
        MetaSeparator::paint(painter, pos2(drag.right() + Self::HINT_GAP + MetaSeparator::SIDE / 2.0, centre));
        let zoom = Text::message(StratumMessage::PreviewHintZoom);
        let at = pos2(drag.right() + 2.0 * Self::HINT_GAP + MetaSeparator::SIDE, centre);
        Draw::text(painter, at, Align2::LEFT_CENTER, &zoom, Self::TIME, faint, TextFit::Line);
    }
}
