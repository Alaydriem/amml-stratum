use crate::types::ConnectionInfo;

// What the world key reads from the game on one tick. The game side reads the client; a test gives fixed answers
pub trait WorldSource {
    // the address of the client level, which names the loaded world for one stay; None out of a world
    fn level(&self) -> Option<usize>;

    fn in_realm(&self) -> bool;

    // the game's connection info; None when the game has none or the entry is off
    fn connection(&self) -> Option<ConnectionInfo>;

    // the last path part of the folder of the local world now open, as the world storage was made with; None when
    // no storage was seen for this world or an entry it needs is off
    fn local_folder(&self) -> Option<String>;

    // the level name of the level data; None when it cannot be read
    fn level_name(&self) -> Option<String>;

    // the 64-bit level seed; None when it cannot be read
    fn seed(&self) -> Option<u64>;
}
