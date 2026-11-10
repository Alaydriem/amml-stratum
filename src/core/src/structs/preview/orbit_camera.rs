// The turn of the preview camera around the structure's centre, in degrees, and the zoom over the fitted start
// view. Yaw 0 looks from +Z towards -Z; pitch is the angle above the horizon.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbitCamera {
    pub yaw: f32,
    pub pitch: f32,
    // 1 is the fitted start view; 2 shows the structure twice as large.
    pub zoom: f32,
}

impl OrbitCamera {
    pub const START_YAW: f32 = 45.0;
    pub const START_PITCH: f32 = 30.0;
    pub const MIN_PITCH: f32 = -60.0;
    pub const MAX_PITCH: f32 = 89.0;
    /// The smallest zoom, against the fitted start view.
    pub const MIN_ZOOM: f32 = 0.5;
    /// The largest zoom, against the fitted start view.
    pub const MAX_ZOOM: f32 = 8.0;
    /// The zoom factor of one wheel step.
    pub const ZOOM_STEP: f32 = 1.15;

    pub fn new(yaw: f32, pitch: f32) -> Self {
        Self { yaw: yaw.rem_euclid(360.0), pitch: pitch.clamp(Self::MIN_PITCH, Self::MAX_PITCH), zoom: 1.0 }
    }

    // The view a preview starts with, at zoom 1.
    pub fn start() -> Self {
        Self::new(Self::START_YAW, Self::START_PITCH)
    }

    // Turns by the given degrees; the pitch stops at its limits and the yaw wraps. The zoom is kept.
    pub fn turned(self, yaw: f32, pitch: f32) -> Self {
        Self { zoom: self.zoom, ..Self::new(self.yaw + yaw, self.pitch + pitch) }
    }

    // Zooms by `steps` wheel steps, in when positive; the zoom stops at its limits. The turn is kept.
    pub fn zoomed(self, steps: i32) -> Self {
        let zoom = (self.zoom * Self::ZOOM_STEP.powi(steps)).clamp(Self::MIN_ZOOM, Self::MAX_ZOOM);
        Self { zoom, ..self }
    }
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self::start()
    }
}
