// What the build conditions read from the game. The game side answers from the client; a test gives fixed answers
pub trait BuildFacts {
    // a world is loaded and has a live local player; the permission call is made only then
    fn in_world(&self) -> bool;

    // the address of the client level, which names the loaded world for one stay; None out of a world
    fn level(&self) -> Option<usize>;

    // the game's connection type of the world; None when the game has none or the entry is off. This copies the
    // game's connection info, so the reader asks once for each world
    fn connection_type(&self) -> Option<i16>;

    fn in_realm(&self) -> bool;

    // the local player's game type; None when it cannot be read
    fn game_type(&self) -> Option<i32>;

    // the local player's permission level (Visitor 0, Member 1, Operator 2, Custom 3); None when it cannot be read
    fn permission_level(&self) -> Option<u8>;

    // the level's commands flag byte (the "Activate Cheats" setting); None when it cannot be read
    fn commands_flag(&self) -> Option<u8>;
}
