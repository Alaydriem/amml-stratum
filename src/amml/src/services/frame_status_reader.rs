use stratum_core::structs::frame::FrameStatus;

use crate::services::WorldKeyReader;
use crate::traits::ClientSource;

// reads the game state the core needs for one tick; read-only, it never changes the game. The status and the world
// key are kept and rewritten in place, so a tick allocates nothing once their texts have grown
pub struct FrameStatusReader {
    world_key: WorldKeyReader,
    // the key of the loaded world, kept between reads; the status shows it only while there is a local player
    key: String,
    status: FrameStatus,
}

impl FrameStatusReader {
    pub fn new() -> Self {
        Self { world_key: WorldKeyReader::new(), key: String::new(), status: FrameStatus::default() }
    }

    // with no active world only the time is set and the key is forgotten; in a world with no local player
    // `in_world` is set, `has_player` is not and the world is empty, since the dimension is not known yet
    pub fn read(&mut self, source: &impl ClientSource, time_seconds: f64) -> &FrameStatus {
        let status = &mut self.status;
        status.in_world = false;
        status.has_player = false;
        status.menu_open = false;
        status.world.clear();
        status.dimension = 0;
        status.feet = [0; 3];
        status.time_seconds = time_seconds;
        status.render_distance = 0;
        if !source.is_world_active() {
            self.world_key.reset();
            self.key.clear();
            return &self.status;
        }
        status.in_world = true;
        self.world_key.read(source, &mut self.key);
        let Some(player) = source.player() else { return &self.status };
        status.world.push_str(&self.key);
        status.has_player = true;
        status.menu_open = source.menu_open();
        status.dimension = player.dimension;
        status.feet = player.feet;
        // the video setting, in chunks; the ghost draws this far unless the config names its own distance
        status.render_distance = u32::try_from(source.view_distance_chunks()).unwrap_or(0);
        &self.status
    }

    pub fn world_key(&self) -> &WorldKeyReader {
        &self.world_key
    }

    // forgets the loaded world, its key and the key logged last
    pub fn reset(&mut self) {
        self.world_key.forget();
        self.key.clear();
    }
}

impl Default for FrameStatusReader {
    fn default() -> Self {
        Self::new()
    }
}
