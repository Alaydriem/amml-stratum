use super::SectionKey;
use super::draw::GhostDraw;
use super::slot_state::SlotState;

// The scheduler's record of one mesh slot.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Slot {
    pub key: Option<SectionKey>,
    pub state: SlotState,
    pub origin: [i32; 3],
    // Bytes the last good report gave.
    pub bytes: u64,
    // A good report arrived at least once, so the slot holds a mesh to draw.
    pub has_mesh: bool,
    // A change arrived while the job was pending.
    pub dirty: bool,
    // The serial of the job in flight or last reported.
    pub serial: u32,
    // The plan count when the job was handed out.
    pub issued_at: u32,
    // Scratch for the plan: the key is on the wanted list.
    pub seen: bool,
    // The slot holds a mesh of the previous build or view. It has no key, is drawn with `drawn`
    // until the wanted keys all have a mesh, and is then freed.
    pub retired: bool,
    // The draw the slot last got.
    pub drawn: Option<GhostDraw>,
    // The scheduler's change count when the slot got its first job.
    pub change: u32,
    // A report for the slot arrived, whatever its mode.
    pub reported: bool,
    // The serial of the first job of the group build in progress. Zero for a Replace slot.
    pub group_serial: u32,
    // The index the next job of the group build gets; the job in flight holds the one before. Zero
    // for a Replace slot.
    pub group_next: u32,
    // The last job of the group build was answered, so the next one may go out.
    pub answered: bool,
    // The job in flight is the Finish job.
    pub finishing: bool,
    // How many times the job at the current group index waited `PENDING_PLANS` plans and went out
    // again. Zero when the index moves or the build starts again.
    pub group_expiries: u32,
}

impl Slot {
    pub fn new() -> Self {
        Self {
            key: None,
            state: SlotState::Free,
            origin: [0; 3],
            bytes: 0,
            has_mesh: false,
            dirty: false,
            serial: 0,
            issued_at: 0,
            seen: false,
            retired: false,
            drawn: None,
            change: 0,
            reported: false,
            group_serial: 0,
            group_next: 0,
            answered: false,
            finishing: false,
            group_expiries: 0,
        }
    }

    // Handed out and not yet reported once.
    pub fn unreported(&self) -> bool {
        self.state == SlotState::Pending && !self.reported
    }

    // The slot builds or built a merged group mesh.
    pub fn is_group(&self) -> bool {
        self.group_next > 0
    }

    // The member index of the next group job and whether it starts the build again, or None when
    // no job may go out. `expired` is true when the job in flight waited `PENDING_PLANS` plans;
    // `idle` is true when no group builds. The first expiry at an index sends the job again, the
    // second starts the build again at index 0.
    pub fn group_step(&self, expired: bool, idle: bool) -> Option<(u32, bool)> {
        match self.state {
            SlotState::Stale if idle => Some((0, true)),
            SlotState::Pending if self.dirty && (self.answered || expired) => Some((0, true)),
            SlotState::Pending if self.answered => Some((self.group_next, false)),
            SlotState::Pending if expired && self.group_expiries >= 1 => Some((0, true)),
            SlotState::Pending if expired => Some((self.group_next.saturating_sub(1), false)),
            _ => None,
        }
    }
}
