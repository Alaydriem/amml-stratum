use crate::traits::WorldSource;
use crate::types::PlayerSpot;

// What the frame status reads from the game on one tick, besides the world key
pub trait ClientSource: WorldSource {
    fn is_world_active(&self) -> bool;

    // the local player's dimension and feet block; None while there is no local player
    fn player(&self) -> Option<PlayerSpot>;

    fn menu_open(&self) -> bool;

    // the video setting, in chunks
    fn view_distance_chunks(&self) -> i32;
}
