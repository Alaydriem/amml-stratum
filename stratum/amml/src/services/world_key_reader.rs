use crate::traits::WorldSource;
use crate::types::{ConnectionInfo, KeyKind};

// names the loaded world with a key that stays the same for one world across game restarts and differs between
// worlds: `local:<storage folder>`, `server:<address>:<port>`, `realm:<level name>` or `name:<level name>:<seed>`.
// It reads the game again while the client level is new and no kind was taken, and, for a server: or name: key,
// during the first `REALM_RECHECK_READS` reads in case the game says late that it is in a Realm. A Realm key needs
// a non-empty level name, and a server: key is taken only after the same key, out of a Realm, on
// `SERVER_STABLE_READS` reads in a row. A world with neither a local nor a server source takes the name key after
// `FALLBACK_AFTER_READS` reads. A chosen key is logged at info level when it is not the key logged last, so a world
// that is read again after a tick out of it is not logged twice. The client level is kept as an address only, never
// followed
pub struct WorldKeyReader {
    // the client level of the last read
    level: Option<usize>,
    kind: KeyKind,
    attempts: u32,
    // the server key read last while it is not yet taken, and how many reads in a row gave it
    candidate: String,
    candidate_reads: u32,
    // the key logged last, and how many keys were logged
    last_logged: String,
    logged: u32,
}

impl WorldKeyReader {
    // reads in one world before a world with neither a local nor a server source takes the name key
    pub const FALLBACK_AFTER_READS: u32 = 300;
    // reads in a row with the same server key, out of a Realm, before that key is taken
    pub const SERVER_STABLE_READS: u32 = 20;
    // reads in one world during which a server: or name: key is given up when the game turns to a Realm
    pub const REALM_RECHECK_READS: u32 = 600;

    pub const LOCAL_PREFIX: &'static str = "local:";
    pub const SERVER_PREFIX: &'static str = "server:";
    pub const REALM_PREFIX: &'static str = "realm:";
    pub const NAME_PREFIX: &'static str = "name:";

    pub fn new() -> Self {
        Self {
            level: None,
            kind: KeyKind::Undecided,
            attempts: 0,
            candidate: String::new(),
            candidate_reads: 0,
            last_logged: String::new(),
            logged: 0,
        }
    }

    // writes the key of the world `source` shows into `key`; empty outside a world and while the kind is still
    // being decided. `key` must hold what the last read wrote: a taken key is not written again
    pub fn read(&mut self, source: &impl WorldSource, key: &mut String) {
        let Some(level) = source.level() else {
            self.reset();
            key.clear();
            return;
        };
        if self.level != Some(level) {
            self.reset();
            self.level = Some(level);
        }
        // local: and realm: are final; server: and name: give way to a Realm during the first reads
        if matches!(self.kind, KeyKind::Local | KeyKind::Realm) {
            return;
        }
        if self.kind != KeyKind::Undecided && self.attempts >= Self::REALM_RECHECK_READS {
            return;
        }
        self.attempts += 1;
        let in_realm = source.in_realm();
        if in_realm {
            self.candidate_reads = 0;
            self.candidate.clear();
            if Self::read_realm(source, &mut self.candidate) {
                key.clear();
                key.push_str(&self.candidate);
                self.kind = KeyKind::Realm;
                self.log_chosen(key, in_realm);
                return;
            }
            // a Realm whose level name is not known yet never keeps a server key
            if self.kind == KeyKind::Server {
                self.kind = KeyKind::Undecided;
                key.clear();
            }
            if self.kind != KeyKind::Undecided {
                return;
            }
            key.clear();
            if self.attempts < Self::FALLBACK_AFTER_READS {
                return;
            }
            self.take_name(source, key, in_realm);
            return;
        }
        if self.kind != KeyKind::Undecided {
            return;
        }
        key.clear();
        let connection = source.connection();
        if Self::read_local(source, connection.as_ref(), key) {
            self.kind = KeyKind::Local;
            self.log_chosen(key, in_realm);
            return;
        }
        if Self::read_server(connection.as_ref(), key) {
            if *key == self.candidate {
                self.candidate_reads += 1;
            } else {
                self.candidate.clear();
                self.candidate.push_str(key);
                self.candidate_reads = 1;
            }
            if self.candidate_reads < Self::SERVER_STABLE_READS && self.attempts < Self::FALLBACK_AFTER_READS {
                key.clear();
                return;
            }
            self.kind = KeyKind::Server;
            self.log_chosen(key, in_realm);
            return;
        }
        self.candidate_reads = 0;
        self.candidate.clear();
        if self.attempts < Self::FALLBACK_AFTER_READS {
            return;
        }
        self.take_name(source, key, in_realm);
    }

    // forgets the last world, so the next read in a world reads the game again
    pub fn reset(&mut self) {
        self.level = None;
        self.kind = KeyKind::Undecided;
        self.attempts = 0;
        self.candidate_reads = 0;
        self.candidate.clear();
    }

    // forgets the last world and the key logged last, so the next chosen key is logged
    pub fn forget(&mut self) {
        self.reset();
        self.last_logged.clear();
    }

    // how many chosen keys were logged
    pub fn logged(&self) -> u32 {
        self.logged
    }

    // a local connection, or one to a loopback address, names the world by the folder of its storage; no connection
    // info counts as local too
    fn read_local(source: &impl WorldSource, connection: Option<&ConnectionInfo>, key: &mut String) -> bool {
        if let Some(connection) = connection
            && connection.connection_type != ConnectionInfo::LOCAL
            && !Self::is_loopback(&connection.host)
            && !Self::is_loopback(&connection.url)
        {
            return false;
        }
        let Some(folder) = source.local_folder().filter(|folder| !folder.is_empty()) else { return false };
        key.push_str(Self::LOCAL_PREFIX);
        key.push_str(&folder);
        true
    }

    // the unresolved URL, or the host when there is none, and the port; an address with a colon that is not already
    // in brackets is bracketed
    fn read_server(connection: Option<&ConnectionInfo>, key: &mut String) -> bool {
        let Some(connection) = connection else { return false };
        if connection.connection_type == ConnectionInfo::LOCAL
            || connection.connection_type == ConnectionInfo::UNDEFINED
        {
            return false;
        }
        let address = if connection.url.is_empty() { &connection.host } else { &connection.url };
        if address.is_empty() || connection.port <= 0 {
            return false;
        }
        let bracket = address.contains(':') && !address.starts_with('[');
        key.push_str(Self::SERVER_PREFIX);
        if bracket {
            key.push('[');
        }
        key.push_str(address);
        if bracket {
            key.push(']');
        }
        key.push(':');
        key.push_str(&connection.port.to_string());
        true
    }

    // false while the level name is empty
    fn read_realm(source: &impl WorldSource, key: &mut String) -> bool {
        let Some(name) = source.level_name().filter(|name| !name.is_empty()) else { return false };
        key.push_str(Self::REALM_PREFIX);
        key.push_str(&name);
        true
    }

    // takes the name key once both its parts can be read; until then the key stays empty and the next read tries
    // again
    fn take_name(&mut self, source: &impl WorldSource, key: &mut String, in_realm: bool) {
        if Self::read_name(source, key) {
            self.kind = KeyKind::Name;
            self.log_chosen(key, in_realm);
        }
    }

    // the level name as the game holds it, an empty one too, and the seed in unsigned decimal, as the game's uint64
    // prints; false, writing nothing, while either cannot be read
    fn read_name(source: &impl WorldSource, key: &mut String) -> bool {
        let (Some(name), Some(seed)) = (source.level_name(), source.seed()) else { return false };
        key.push_str(Self::NAME_PREFIX);
        key.push_str(&name);
        key.push(':');
        key.push_str(&seed.to_string());
        true
    }

    fn is_loopback(address: &str) -> bool {
        address == "localhost" || address == "::1" || address == "[::1]" || address.starts_with("127.")
    }

    fn log_chosen(&mut self, key: &str, in_realm: bool) {
        if self.last_logged == key {
            return;
        }
        self.last_logged.clear();
        self.last_logged.push_str(key);
        self.logged += 1;
        let kind = key.split(':').next().unwrap_or_default();
        curia::info!("world key", { kind: kind, key: key, in_realm: in_realm, reads: self.attempts });
    }
}

impl Default for WorldKeyReader {
    fn default() -> Self {
        Self::new()
    }
}
