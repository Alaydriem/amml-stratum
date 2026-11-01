// What the game reports on one frame. `time_seconds` is a steady clock that only moves forward.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FrameStatus {
    pub in_world: bool,
    pub has_player: bool,
    pub menu_open: bool,
    pub world: String,
    pub dimension: i32,
    pub feet: [i32; 3],
    pub time_seconds: f64,
    // The game's render distance in chunks.
    pub render_distance: u32,
}
