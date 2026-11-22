use amml_api::{ConnectionKind, GameType, PermissionLevel};

// What the build conditions read from the game. The game side answers from the client; a test gives fixed answers
pub trait BuildFacts {
    // a world is joined and has a local player
    fn in_world(&self) -> bool;

    // the loader's epoch of the joined world, which names the world for one stay; None out of a world
    fn level(&self) -> Option<u64>;

    // the kind of the world's connection; None while the game has none
    fn connection_kind(&self) -> Option<ConnectionKind>;

    // the game says the world is a Realm
    fn in_realm(&self) -> bool;

    // the local player's game type; None when it cannot be read
    fn game_type(&self) -> Option<GameType>;

    // the local player's permission level; None when it cannot be read
    fn permission_level(&self) -> Option<PermissionLevel>;

    // the world's commands setting (the "Activate Cheats" toggle); None when it cannot be read
    fn commands_enabled(&self) -> Option<bool>;
}
