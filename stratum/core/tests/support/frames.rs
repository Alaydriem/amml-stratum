use stratum_core::structs::frame::FrameStatus;

// Frame statuses for controller tests. Time is always an argument; nothing reads a clock.
pub struct Frames;

impl Frames {
    pub fn in_world(world: &str, time_seconds: f64) -> FrameStatus {
        Self::at(world, [0, 64, 0], time_seconds)
    }

    pub fn at(world: &str, feet: [i32; 3], time_seconds: f64) -> FrameStatus {
        FrameStatus {
            in_world: true,
            has_player: true,
            menu_open: false,
            world: world.to_string(),
            dimension: 0,
            feet,
            time_seconds,
            render_distance: 12,
        }
    }

    pub fn in_dimension(world: &str, dimension: i32, time_seconds: f64) -> FrameStatus {
        FrameStatus { dimension, ..Self::in_world(world, time_seconds) }
    }

    pub fn main_menu(time_seconds: f64) -> FrameStatus {
        FrameStatus { menu_open: true, time_seconds, ..FrameStatus::default() }
    }
}
