use stratum_core::structs::ghost::ChestPair;

use super::block_actor_key::BlockActorKey;

/// A fixed open-addressed map from a key to its slot number in the pool.
///
/// `SLOTS` is twice the pool capacity, so the table is at most half full. Probing is linear and deletion shifts
/// the following entries back, so there are no tombstones. The table holds slot numbers only; the keys live in the
/// pool and are passed to every call. It is allocated once in `new`.
pub struct BlockActorIndex {
    table: Vec<u32>,
}

impl BlockActorIndex {
    pub const SLOTS: usize = 512;
    const MASK: usize = Self::SLOTS - 1;
    const EMPTY: u32 = u32::MAX;

    pub fn new() -> Self {
        Self { table: vec![Self::EMPTY; Self::SLOTS] }
    }

    // home position of a key in the table
    fn home(key: &BlockActorKey) -> usize {
        let chest: u64 = match key.chest {
            None => 0,
            Some(ChestPair::PlusX) => 1,
            Some(ChestPair::MinusX) => 2,
            Some(ChestPair::PlusZ) => 3,
            Some(ChestPair::MinusZ) => 4,
        };
        let packed = chest << 48 | u64::from(key.entry) << 16 | u64::from(key.palette);
        let mixed = packed.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        (mixed >> 32) as usize & Self::MASK
    }

    /// Slot number of `key`, when present. `keys` maps slot numbers to keys.
    pub fn find(&self, keys: &[BlockActorKey], key: &BlockActorKey) -> Option<usize> {
        let mut at = Self::home(key);
        for _ in 0..Self::SLOTS {
            let slot = *self.table.get(at)?;
            if slot == Self::EMPTY {
                return None;
            }
            if keys.get(slot as usize) == Some(key) {
                return Some(slot as usize);
            }
            at = (at + 1) & Self::MASK;
        }
        None
    }

    /// Records `key` at `slot`. The key must not be present and `keys[slot]` must already hold it.
    /// Returns false when the table has no free entry.
    pub fn insert(&mut self, key: &BlockActorKey, slot: usize) -> bool {
        let mut at = Self::home(key);
        for _ in 0..Self::SLOTS {
            match self.table.get_mut(at) {
                Some(entry) if *entry == Self::EMPTY => {
                    *entry = slot as u32;
                    return true;
                }
                Some(_) => at = (at + 1) & Self::MASK,
                None => return false,
            }
        }
        false
    }

    /// Removes `key`. `keys` must still hold every key in the table, including this one.
    pub fn remove(&mut self, keys: &[BlockActorKey], key: &BlockActorKey) {
        let mut hole = Self::home(key);
        let mut found = false;
        for _ in 0..Self::SLOTS {
            let Some(&slot) = self.table.get(hole) else { return };
            if slot == Self::EMPTY {
                return;
            }
            if keys.get(slot as usize) == Some(key) {
                found = true;
                break;
            }
            hole = (hole + 1) & Self::MASK;
        }
        if !found {
            return;
        }
        let mut next = hole;
        loop {
            next = (next + 1) & Self::MASK;
            let Some(&slot) = self.table.get(next) else { break };
            if slot == Self::EMPTY {
                break;
            }
            let Some(next_key) = keys.get(slot as usize) else { break };
            let home = Self::home(next_key);
            // the entry may move into the hole only when its home is not cyclically within (hole, next]
            let stays = if hole <= next { hole < home && home <= next } else { hole < home || home <= next };
            if !stays {
                if let Some(entry) = self.table.get_mut(hole) {
                    *entry = slot;
                }
                hole = next;
            }
        }
        if let Some(entry) = self.table.get_mut(hole) {
            *entry = Self::EMPTY;
        }
    }

    pub fn clear(&mut self) {
        self.table.fill(Self::EMPTY);
    }
}

impl Default for BlockActorIndex {
    fn default() -> Self {
        Self::new()
    }
}
