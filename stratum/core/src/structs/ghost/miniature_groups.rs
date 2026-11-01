use super::{SectionKey, SectionPart};

// The occupied section keys of a build, cut in order into groups of at most `GROUP_CELLS` filled
// cells. The mini ghost draws one merged mesh per group.
//
// Bound: `members` holds at most the occupied key count of the build it was cut from, and is
// emptied by `clear`. `starts` holds one entry per group, which is at most that count.
#[derive(Clone, PartialEq, Eq)]
pub struct MiniatureGroups {
    members: Vec<SectionKey>,
    // The index in `members` where each group starts.
    starts: Vec<usize>,
}

impl MiniatureGroups {
    // Filled cells one group holds at most. A key that holds more is a group of its own.
    pub const GROUP_CELLS: u32 = 16_384;

    pub fn new() -> Self {
        Self { members: Vec::new(), starts: Vec::new() }
    }

    // Replaces the groups with a cut of `occupied`; `cells[i]` is the cell count of `occupied[i]`.
    // A key opens a new group when adding it would pass `GROUP_CELLS`.
    pub fn cut(&mut self, occupied: &[SectionKey], cells: &[u32]) {
        debug_assert_eq!(occupied.len(), cells.len());
        self.clear();
        let mut total = 0_u32;
        for (index, (key, count)) in occupied.iter().zip(cells).enumerate() {
            if index == 0 || total.saturating_add(*count) > Self::GROUP_CELLS {
                self.starts.push(index);
                total = 0;
            }
            total = total.saturating_add(*count);
            self.members.push(*key);
        }
    }

    pub fn clear(&mut self) {
        self.members.clear();
        self.starts.clear();
    }

    pub fn group_count(&self) -> u32 {
        self.starts.len() as u32
    }

    // The keys of `group`, empty past the last group.
    pub fn members(&self, group: u32) -> &[SectionKey] {
        let group = group as usize;
        let Some(start) = self.starts.get(group) else {
            return &[];
        };
        let end = self.starts.get(group + 1).copied().unwrap_or(self.members.len());
        &self.members[*start..end]
    }

    // The key of the merged mesh of `group`.
    pub fn key(group: u32) -> SectionKey {
        SectionKey { x: group as i32, y: 0, z: 0, layer: None, part: SectionPart::MiniatureGroup }
    }

    // One key per group, in order.
    pub fn keys(&self) -> impl Iterator<Item = SectionKey> + use<> {
        (0..self.group_count()).map(Self::key)
    }
}

impl Default for MiniatureGroups {
    fn default() -> Self {
        Self::new()
    }
}
