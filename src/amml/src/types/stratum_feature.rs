use amml_game::EntryId;

use crate::types::StratumEntry;

// the optional features of stratum and the entries each one needs; everything else is required. With the connection
// or the world folder off, no local world is named by its folder, and the world key falls back to the level name and
// seed
pub struct StratumFeature;

impl StratumFeature {
    // the connection info: local and server keys
    pub const CONNECTION: &'static str = "stratum.connection";
    // the world storage hooks: the folder of a local world, noted when its storage is made and forgotten when it
    // shuts down
    pub const WORLD_FOLDER: &'static str = "stratum.world_folder";

    // the block-entity ghosts: the game's block actors drawn at the ghost cells. Off, the block ghosts still draw
    pub const BLOCK_ACTORS: &'static str = "stratum.block_actors";

    // the large chests: the lead of a chest pair drawn as a large chest. Off, a lead chest draws as a single chest
    pub const LARGE_CHESTS: &'static str = "stratum.large_chests";

    // the skull ghosts: the skull hook and the ghost's skull pose. Off, the hook is not installed and skull cells are
    // not drawn
    pub const SKULLS: &'static str = "stratum.skulls";

    // the biome water colour of water ghosts. Off, water keeps the texture's own grey
    pub const WATER_TINT: &'static str = "stratum.water_tint";

    pub const OPTIONAL: [&'static str; 6] =
        [Self::CONNECTION, Self::WORLD_FOLDER, Self::BLOCK_ACTORS, Self::LARGE_CHESTS, Self::SKULLS, Self::WATER_TINT];

    // the entries of `feature`; none for a name that is not an optional feature
    pub fn entries(feature: &str) -> &'static [EntryId] {
        match feature {
            Self::CONNECTION => &[StratumEntry::CLIENT_INSTANCE_GET_GAME_CONNECTION_INFO],
            Self::WORLD_FOLDER => &[StratumEntry::DB_STORAGE_CTOR, StratumEntry::DB_STORAGE_CHECK_SHUTDOWN_DONE],
            Self::BLOCK_ACTORS => &StratumEntry::BLOCK_ACTORS,
            Self::LARGE_CHESTS => &StratumEntry::LARGE_CHESTS,
            Self::SKULLS => &StratumEntry::SKULLS,
            Self::WATER_TINT => &StratumEntry::WATER_TINT,
            _ => &[],
        }
    }
}
