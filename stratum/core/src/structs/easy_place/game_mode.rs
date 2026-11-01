// The player's game mode, as far as easy place cares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    Survival,
    Creative,
    // Adventure and spectator. Easy place passes every use.
    Other,
}
