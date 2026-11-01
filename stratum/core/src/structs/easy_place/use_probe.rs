use super::{GameMode, SwapAnswer, UseHit};

// What the shim reads from the game on one use call.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UseProbe {
    pub time_seconds: f64,
    // True on the first call of a hold.
    pub pressed: bool,
    pub held: bool,
    pub eye: [f64; 3],
    pub dir: [f64; 3],
    pub reach: f64,
    pub game_mode: GameMode,
    pub hit: Option<UseHit>,
    // The cells the player body fills, min and max inclusive.
    pub body: Option<([i32; 3], [i32; 3])>,
    pub swap_answer: SwapAnswer,
}
