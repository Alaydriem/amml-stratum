// The parts of the game's Social::GameConnectionInfo the world key reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionInfo {
    // the game's Social::ConnectionType: -1 undefined, 0 local, higher values are network connections
    pub connection_type: i16,
    pub host: String,
    pub url: String,
    pub port: i32,
}

impl ConnectionInfo {
    pub const UNDEFINED: i16 = -1;
    pub const LOCAL: i16 = 0;
}
