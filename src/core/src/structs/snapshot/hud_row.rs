use crate::structs::easy_place::EasyPlaceNotice;
use crate::structs::marker::Rotation;
use crate::structs::material::ItemId;

// What the HUD card shows for the active marker. Built by the tick, never by a frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudRow {
    pub marker_name: String,
    // The structure shown: the target structure, the selected structure of a normal project, or the
    // project name of a big build. Empty when the target is gone.
    pub target_name: String,
    // The selected structure of a normal project, 1-based, and the structure count.
    pub position: Option<(u32, u32)>,
    // The piece of a big build that holds the player's feet, else the nearest piece.
    pub piece_name: Option<String>,
    pub layers: u32,
    // The shown layer, 0-based; None when the whole build shows.
    pub layer: Option<u32>,
    // The world Y of the shown layer.
    pub y_level: Option<i32>,
    // The lowest and highest world Y of the build; None for a build with no layers.
    pub y_range: Option<(i32, i32)>,
    pub shown: bool,
    // The opacity step index, 0 to `OpacityStep::COUNT - 1`.
    pub opacity_step: u8,
    pub origin: [i32; 3],
    pub rotation: Rotation,
    // The Structure control has structures to step through.
    pub has_structures: bool,
    // Easy place is on for the marker.
    pub easy_place: bool,
    // The latest easy place notice, kept for a few seconds after it was raised.
    pub notice: Option<EasyPlaceNotice>,
    // The item a missing-block notice asks for; None for any other notice.
    pub notice_item: Option<ItemId>,
}
