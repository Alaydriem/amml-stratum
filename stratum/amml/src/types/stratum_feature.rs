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

    pub const OPTIONAL: [&'static str; 2] = [Self::CONNECTION, Self::WORLD_FOLDER];

    // the entries of `feature`; none for a name that is not an optional feature
    pub fn entries(feature: &str) -> &'static [EntryId] {
        match feature {
            Self::CONNECTION => &[StratumEntry::CLIENT_INSTANCE_GET_GAME_CONNECTION_INFO],
            Self::WORLD_FOLDER => &[StratumEntry::DB_STORAGE_CTOR, StratumEntry::DB_STORAGE_CHECK_SHUTDOWN_DONE],
            _ => &[],
        }
    }
}
