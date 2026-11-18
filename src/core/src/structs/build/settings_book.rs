use super::BuildSettings;
use crate::structs::library::StructureId;

// The settings the panel keeps for each structure. Bound: `capacity` entries; the entry used longest ago is
// dropped when a new one would pass it. A read through `use_settings` or a write counts as a use.
//
// The book also holds the worlds that are owed a game rule restore: a build changed the rule there and ended with
// the world gone, so the rule could not be put back. Bound: `OWED_CAPACITY` worlds; the oldest goes first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildSettingsBook {
    capacity: usize,
    // Oldest use first.
    entries: Vec<(StructureId, BuildSettings)>,
    // The worlds owed a restore, oldest first.
    owed: Vec<String>,
    dirty: bool,
}

impl BuildSettingsBook {
    // The most worlds that are owed a restore at one time.
    pub const OWED_CAPACITY: usize = 16;

    pub fn new(capacity: usize) -> Self {
        Self { capacity: capacity.max(1), entries: Vec::new(), owed: Vec::new(), dirty: false }
    }

    // A book of saved entries, oldest use first. Rates are pulled into range, and when there are more entries
    // than the capacity, the ones used longest ago are dropped.
    pub fn from_entries(capacity: usize, entries: Vec<(StructureId, BuildSettings)>) -> Self {
        let mut book = Self::new(capacity);
        for (id, settings) in entries {
            book.insert(id, settings.clamped());
        }
        book
    }

    // The same book with the worlds that are owed a restore, oldest first. Past `OWED_CAPACITY` the oldest are dropped.
    pub fn with_owed(mut self, worlds: Vec<String>) -> Self {
        for world in worlds {
            self.push_owed(world);
        }
        self.dirty = false;
        self
    }

    // Notes that `world` is owed a restore. A world that is already noted changes nothing.
    pub fn owe(&mut self, world: &str) {
        if !self.owes(world) {
            self.push_owed(world.to_string());
            self.dirty = true;
        }
    }

    // The restore of `world` was sent.
    pub fn clear_owed(&mut self, world: &str) {
        let before = self.owed.len();
        self.owed.retain(|known| known != world);
        self.dirty |= self.owed.len() != before;
    }

    pub fn owes(&self, world: &str) -> bool {
        self.owed.iter().any(|known| known == world)
    }

    // The worlds owed a restore, oldest first.
    pub fn owed(&self) -> &[String] {
        &self.owed
    }

    fn push_owed(&mut self, world: String) {
        self.owed.retain(|known| *known != world);
        if self.owed.len() >= Self::OWED_CAPACITY {
            self.owed.remove(0);
        }
        self.owed.push(world);
    }

    // The settings of a structure, or the defaults. The structure counts as used.
    pub fn use_settings(&mut self, id: StructureId) -> BuildSettings {
        match self.entries.iter().position(|(known, _)| *known == id) {
            Some(index) => {
                let entry = self.entries.remove(index);
                self.entries.push(entry);
                entry.1
            }
            None => BuildSettings::default(),
        }
    }

    // The settings of a structure, or the defaults, without counting a use.
    pub fn get(&self, id: StructureId) -> BuildSettings {
        self.entries.iter().find(|(known, _)| *known == id).map(|(_, settings)| *settings).unwrap_or_default()
    }

    pub fn set(&mut self, id: StructureId, settings: BuildSettings) {
        self.insert(id, settings);
        self.dirty = true;
    }

    // Drops the entry of a deleted structure.
    pub fn remove(&mut self, id: StructureId) {
        let before = self.entries.len();
        self.entries.retain(|(known, _)| *known != id);
        self.dirty |= self.entries.len() != before;
    }

    // The entries, oldest use first.
    pub fn entries(&self) -> &[(StructureId, BuildSettings)] {
        &self.entries
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }

    fn insert(&mut self, id: StructureId, settings: BuildSettings) {
        self.entries.retain(|(known, _)| *known != id);
        if self.entries.len() >= self.capacity {
            self.entries.remove(0);
        }
        self.entries.push((id, settings));
    }
}
