use amml_api::MeshGroup;
use stratum_core::structs::ghost::{GhostJob, GhostJobMode};

use crate::types::GroupStep;

/// The one mini ghost group being built: the loader's open `MeshGroup` that the Append jobs applied so far went into,
/// the job identity it belongs to (slot, group build serial, next index) and the bytes counted for it; and per slot the
/// group build serial and bytes of the group it finished last. A group dropped here is discarded by the loader.
///
/// Bound: one open group at a time, its size held by core's group cell limit; the per-slot records are sized with the
/// slot table.
pub struct GroupTracker {
    group: Option<MeshGroup>,
    slot: u32,
    serial: u32,
    next: u32,
    bytes: u64,
    finished: Vec<Option<u32>>,
    finished_bytes: Vec<u64>,
}

impl GroupTracker {
    pub fn new() -> Self {
        Self { group: None, slot: 0, serial: 0, next: 0, bytes: 0, finished: Vec::new(), finished_bytes: Vec::new() }
    }

    /// Sized with the slot table; holds nothing after.
    pub fn resize(&mut self, slots: usize) {
        self.group = None;
        self.finished.clear();
        self.finished.resize(slots, None);
        self.finished_bytes.clear();
        self.finished_bytes.resize(slots, 0);
    }

    pub fn step(&self, job: &GhostJob) -> GroupStep {
        if self.group.is_some() && job.slot == self.slot && job.group_serial == self.serial {
            return match job.group_index.cmp(&self.next) {
                std::cmp::Ordering::Equal => GroupStep::Apply,
                std::cmp::Ordering::Less => GroupStep::Repeat,
                std::cmp::Ordering::Greater => GroupStep::Ignore,
            };
        }
        let finished = self.finished.get(job.slot as usize).copied().flatten();
        if job.mode == GhostJobMode::Finish && finished == Some(job.group_serial) {
            return GroupStep::RepeatFinish;
        }
        if job.group_index == 0 { GroupStep::Apply } else { GroupStep::Ignore }
    }

    /// Whether a group build is open.
    pub fn is_open(&self) -> bool {
        self.group.is_some()
    }

    /// `group` for the job's slot and group build, in place of any other; the job is at index 0.
    pub fn start(&mut self, job: &GhostJob, group: MeshGroup) {
        self.group = Some(group);
        self.slot = job.slot;
        self.serial = job.group_serial;
        self.next = 0;
        self.bytes = 0;
    }

    /// The open group, for an append.
    pub fn group_mut(&mut self) -> Option<&mut MeshGroup> {
        self.group.as_mut()
    }

    /// An append went into the open group, counted at `bytes`; the next index is due.
    pub fn appended(&mut self, bytes: u64) {
        self.next += 1;
        self.bytes += bytes;
    }

    /// The bytes counted for the open group.
    pub fn bytes(&self) -> u64 {
        self.bytes
    }

    /// The open group, for its finish; the tracker holds none after.
    pub fn take(&mut self) -> Option<MeshGroup> {
        self.group.take()
    }

    /// Records the slot's finished group with what its mesh holds.
    pub fn finish(&mut self, slot: u32, serial: u32, bytes: u64) {
        self.group = None;
        let index = slot as usize;
        if index < self.finished.len() {
            self.finished[index] = Some(serial);
            self.finished_bytes[index] = bytes;
        }
    }

    pub fn finished_bytes(&self, slot: u32) -> u64 {
        self.finished_bytes.get(slot as usize).copied().unwrap_or(0)
    }

    /// Drops the open group.
    pub fn drop_build(&mut self) {
        self.group = None;
    }

    /// Drops the open group if it is for `slot`, and the slot's finished record.
    pub fn free(&mut self, slot: u32) {
        if self.group.is_some() && self.slot == slot {
            self.group = None;
        }
        let index = slot as usize;
        if index < self.finished.len() {
            self.finished[index] = None;
            self.finished_bytes[index] = 0;
        }
    }

    /// Drops the open group and every finished record.
    pub fn clear(&mut self) {
        self.group = None;
        self.finished.fill(None);
        self.finished_bytes.fill(0);
    }
}

impl Default for GroupTracker {
    fn default() -> Self {
        Self::new()
    }
}
