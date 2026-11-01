// Why the library closed or the ghost stopped, for the log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseReason {
    // The open key was pressed again.
    Key,
    // The player closed it from the library itself.
    Frame,
    // The game showed a menu or the pause screen.
    Menu,
    // The player left the world.
    WorldExit,
    // Another mod's exclusive claim suspended the lease.
    Suspended,
    // The manager took the lease back.
    Revoked,
    // The mod was disabled.
    Disabled,
}

impl CloseReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Key => "key",
            Self::Frame => "frame",
            Self::Menu => "menu",
            Self::WorldExit => "world_exit",
            Self::Suspended => "suspended",
            Self::Revoked => "revoked",
            Self::Disabled => "disabled",
        }
    }
}
