#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudMode {
    // Each key has one fixed action.
    Quick,
    // Left and Right pick a control, Up and Down change it.
    Full,
}
