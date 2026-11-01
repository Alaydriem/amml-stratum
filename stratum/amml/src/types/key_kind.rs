// What kind of key the world key reader took for the loaded world; `Undecided` while it still reads the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyKind {
    Undecided,
    Local,
    Server,
    Realm,
    Name,
}
