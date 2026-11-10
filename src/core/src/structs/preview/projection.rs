use super::{OrbitCamera, PreviewBounds};

// A perspective camera that looks at the centre of a box from the orbit camera's turn, far enough away that the
// box's bounding sphere fits the shorter side of the image. The box is the drawn blocks of the mesh.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreviewProjection {
    eye: [f32; 3],
    right: [f32; 3],
    up: [f32; 3],
    forward: [f32; 3],
    focal: f32,
    centre_x: f32,
    centre_y: f32,
}

impl PreviewProjection {
    pub const FOV_DEGREES: f32 = 35.0;
    // Points closer to the eye than this are not drawn.
    pub const NEAR: f32 = 0.05;
    // The share of the half image the structure reaches from the start view.
    pub const FILL: f32 = 0.92;

    // The box fills `FILL` of the image from the start view; the same zoom is kept for every other turn, so the
    // size does not change while the camera orbits. The zoom never lets the bounding sphere outgrow the longer side
    // of the image. The camera's zoom then scales that fitted view about the image centre.
    pub fn new(bounds: PreviewBounds, camera: OrbitCamera, width: u32, height: u32) -> Self {
        let start = Self::build(bounds, OrbitCamera::start(), width, height, 1.0);
        let (half_w, half_h) = (width.max(1) as f32 / 2.0, height.max(1) as f32 / 2.0);
        let (low, high) = (bounds.low(), bounds.high());
        let mut reach: f32 = 0.0;
        for corner in 0..8u32 {
            let point = [
                if corner & 1 == 0 { low[0] } else { high[0] },
                if corner & 2 == 0 { low[1] } else { high[1] },
                if corner & 4 == 0 { low[2] } else { high[2] },
            ];
            let [x, y, inverse] = start.project(point);
            if inverse > 0.0 {
                reach = reach.max(((x - start.centre_x) / half_w).abs()).max(((y - start.centre_y) / half_h).abs());
            }
        }
        let longest = half_w.max(half_h) / half_w.min(half_h);
        let fit = if reach > 0.0 { (Self::FILL / reach).clamp(1.0, longest) } else { 1.0 };
        Self::build(bounds, camera, width, height, fit * camera.zoom)
    }

    fn build(bounds: PreviewBounds, camera: OrbitCamera, width: u32, height: u32, zoom: f32) -> Self {
        let (low, high) = (bounds.low(), bounds.high());
        let extent = [high[0] - low[0], high[1] - low[1], high[2] - low[2]];
        let target = [low[0] + extent[0] / 2.0, low[1] + extent[1] / 2.0, low[2] + extent[2] / 2.0];
        let radius = (extent[0] * extent[0] + extent[1] * extent[1] + extent[2] * extent[2]).sqrt().max(1.0) / 2.0;
        let half_fov = (Self::FOV_DEGREES / 2.0).to_radians();
        let distance = radius / half_fov.sin();
        let (yaw, pitch) = (camera.yaw.to_radians(), camera.pitch.to_radians());
        let back = [pitch.cos() * yaw.sin(), pitch.sin(), pitch.cos() * yaw.cos()];
        let eye = [target[0] + back[0] * distance, target[1] + back[1] * distance, target[2] + back[2] * distance];
        let forward = back.map(|axis| -axis);
        let right = Self::normalise(Self::cross(forward, [0.0, 1.0, 0.0]));
        let up = Self::cross(right, forward);
        let focal = zoom * 0.5 * width.min(height).max(1) as f32 / half_fov.tan();
        Self { eye, right, up, forward, focal, centre_x: width as f32 / 2.0, centre_y: height as f32 / 2.0 }
    }

    pub fn eye(&self) -> [f32; 3] {
        self.eye
    }

    // Screen x and y in pixels and 1 / depth for a world point; the last value is at or under zero when the point
    // is behind the near plane.
    #[inline]
    pub fn project(&self, point: [f32; 3]) -> [f32; 3] {
        let relative = [point[0] - self.eye[0], point[1] - self.eye[1], point[2] - self.eye[2]];
        let depth = Self::dot(relative, self.forward);
        if depth < Self::NEAR {
            return [0.0, 0.0, 0.0];
        }
        let inverse = 1.0 / depth;
        let x = self.centre_x + self.focal * Self::dot(relative, self.right) * inverse;
        let y = self.centre_y - self.focal * Self::dot(relative, self.up) * inverse;
        [x, y, inverse]
    }

    #[inline]
    fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
        a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
    }

    fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
    }

    fn normalise(a: [f32; 3]) -> [f32; 3] {
        let length = Self::dot(a, a).sqrt();
        if length <= f32::EPSILON { [1.0, 0.0, 0.0] } else { a.map(|axis| axis / length) }
    }
}
