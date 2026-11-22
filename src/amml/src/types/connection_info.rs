use amml_api::{Connection, ConnectionKind};

// The parts of the world's connection the world key reads: the kind, the resolved host and the port the loader reads
// from the game, and the address as the player typed it, from the server join that started the world (empty for a
// world no server join started).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionInfo {
    pub kind: ConnectionKind,
    pub host: String,
    pub url: String,
    pub port: u16,
}

impl From<Connection> for ConnectionInfo {
    fn from(connection: Connection) -> Self {
        Self { kind: connection.kind, host: connection.host, url: String::new(), port: connection.port }
    }
}
