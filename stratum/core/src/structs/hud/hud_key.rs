// A HUD key press, after the shim has mapped the bound game keys to these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudKey {
    Up,
    Down,
    Left,
    Right,
    Mode,
}
