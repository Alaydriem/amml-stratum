use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::config::GhostConfig;
use crate::services::GhostSections;
use crate::structs::ghost::slot::Slot;
use crate::structs::ghost::slot_state::SlotState;
use crate::structs::ghost::{
    GhostBuild, GhostDraw, GhostFrame, GhostJob, GhostJobMode, GhostPalette, GhostReport, MiniatureGroups, SectionKey,
    SectionPart,
};

// Decides which wanted sections hold a mesh slot, which jobs run this frame and which slots go.
//
// `slots`, `by_key`, `free` and `pending_frees` are each bounded by `slot_capacity`, which the
// mesh budget sets. A slot leaves them when its key stops being wanted, when a nearer wanted key
// takes it, or when the build generation changes. `failed` is bounded by `MAX_FAILED` and is
// emptied on a new generation; past the bound a failure is not remembered and the key is retried.
// `spare` holds the job objects of the last frame, at most one per slot, and `carry` at most one
// more.
//
// Every draw comes from `GhostSections::draw`, which places a full part at its built origin and a
// mini ghost group scaled about the build's lowest corner. It is then moved by `shift`, so a moved
// build is drawn at its new place before its meshes are built again there. `Slot::drawn` keeps the
// draw without the shift.
//
// The first `undrawn_lead` keys of the wanted list are built and kept like any other key but are not
// drawn, which is how the mini ghost stays built while the view hides it.
//
// A `MiniatureGroup` key is built by one Append job per member section and then one Finish job, with
// one job in flight. `building` names the one slot whose group builds, so at most one group is
// built at a time; it is bounded by one slot and emptied when that slot is released, retired or
// reset. The next job of a group goes out only after the last was answered; a job that waits
// `PENDING_PLANS` plans is handed out again, and one that waits twice at the same index starts the
// group build again at index 0 under a new `group_serial` (`Slot::group_expiries`).
//
// A change of the build or the view retires the slots that hold a mesh instead of freeing them.
// Retired slots take room in the slot table and the budget. They stay drawn, and the new meshes
// stay hidden, until every wanted key has a mesh; a key that needs a slot when none is free frees
// a retired one first.
pub struct GhostScheduler {
    capacity: u32,
    budget_bytes: u64,
    cells_per_frame: u32,
    generation: Option<u64>,
    slots: Vec<Slot>,
    by_key: HashMap<SectionKey, u32>,
    free: Vec<u32>,
    pending_frees: Vec<u32>,
    failed: HashSet<SectionKey>,
    spare: Vec<GhostJob>,
    // A filled job that passed the cell budget, kept for the plan that reaches its key.
    carry: Option<GhostJob>,
    used_bytes: u64,
    plans: u32,
    next_serial: u32,
    // Slots handed out whose first report has not arrived.
    unreported: u32,
    // Slots with `retired` set.
    retired: u32,
    // The next plan retires the slots whose key left the wanted list, instead of freeing them.
    hold_unwanted: bool,
    // False while the caller's wanted list is still growing; the retired slots stay meanwhile.
    wanted_complete: bool,
    // Moves on each change that retires slots. Slots that got their first job since the last
    // change are not drawn while retired slots are.
    change: u32,
    // How many leading keys of the wanted list get a slot and jobs but no draw.
    undrawn_lead: usize,
    // The slot whose group is being built.
    building: Option<u32>,
}

impl GhostScheduler {
    // About 870 filled cells at about 300 bytes of mesh each. A key holds up to 2 x cells_per_frame
    // cells, or a half up to 2,048, and the target build averages about 700 per key; the GPU cost
    // measured about 192 bytes per cell. At the default 1024 MiB this gives 4,096 slots. The
    // reported bytes, not this estimate, keep the meshes inside the budget.
    pub const SLOT_BYTES_ESTIMATE: u64 = 256 * 1024;

    // Most failed keys remembered in one generation.
    pub const MAX_FAILED: usize = 4096;

    // Plans a job may stay without a report before it is handed out again.
    pub const PENDING_PLANS: u32 = 8;

    pub fn new(config: &GhostConfig) -> Self {
        let budget_bytes = config.mesh_budget_bytes();
        let capacity = u32::try_from(budget_bytes / Self::SLOT_BYTES_ESTIMATE).unwrap_or(u32::MAX).max(1);
        let mut scheduler = Self {
            capacity,
            budget_bytes,
            cells_per_frame: config.cells_per_frame(),
            generation: None,
            slots: vec![Slot::new(); capacity as usize],
            by_key: HashMap::new(),
            free: Vec::with_capacity(capacity as usize),
            pending_frees: Vec::with_capacity(capacity as usize),
            failed: HashSet::new(),
            spare: Vec::new(),
            carry: None,
            used_bytes: 0,
            plans: 0,
            next_serial: 1,
            unreported: 0,
            retired: 0,
            hold_unwanted: false,
            wanted_complete: true,
            change: 0,
            undrawn_lead: 0,
            building: None,
        };
        scheduler.reset();
        scheduler
    }

    pub fn slot_capacity(&self) -> u32 {
        self.capacity
    }

    // Reported bytes of every live slot.
    pub fn used_bytes(&self) -> u64 {
        self.used_bytes
    }

    pub fn live_slots(&self) -> u32 {
        self.capacity - self.free.len() as u32
    }

    // The build revision the slots belong to; None after a plan with no build.
    pub fn generation(&self) -> Option<u64> {
        self.generation
    }

    // A new frame allowance, for the next plan on.
    pub fn set_cells_per_frame(&mut self, cells_per_frame: u32) {
        self.cells_per_frame = cells_per_frame;
    }

    // The first `count` keys of each later `wanted` list keep their slots and jobs but get no draw.
    // Stays set until changed.
    pub fn set_undrawn_lead(&mut self, count: usize) {
        self.undrawn_lead = count;
    }

    // Finished group slots, their member sections and the bytes they reported.
    pub fn miniature_group_slots(&self, groups: &MiniatureGroups) -> (u32, u32, u64) {
        self.slots
            .iter()
            .filter(|slot| slot.has_mesh)
            .filter_map(|slot| {
                slot.key.filter(|key| key.part == SectionPart::MiniatureGroup).map(|key| (key, slot.bytes))
            })
            .fold((0, 0, 0), |(count, sections, bytes), (key, slot_bytes)| {
                (count + 1, sections + groups.members(key.x as u32).len() as u32, bytes + slot_bytes)
            })
    }

    // Fills `frame` with nothing to free, build or draw, and keeps every slot and the generation, so
    // the next plan draws the built slots again at once.
    pub fn pause(&mut self, frame: &mut GhostFrame) {
        self.spare.append(&mut frame.jobs);
        frame.draws.clear();
        frame.frees.clear();
        frame.clear_all = false;
        frame.cut = false;
        frame.palette = None;
        frame.slot_capacity = self.capacity;
        frame.generation = self.generation.unwrap_or(0);
    }

    // Marks built slots for a rebuild into the same slot.
    pub fn invalidate(&mut self, keys: &[SectionKey]) {
        for key in keys {
            let Some(&id) = self.by_key.get(key) else {
                continue;
            };
            let slot = &mut self.slots[id as usize];
            match slot.state {
                SlotState::Built => slot.state = SlotState::Stale,
                SlotState::Pending => slot.dirty = true,
                SlotState::Stale | SlotState::Free => {}
            }
        }
    }

    // Marks every built slot for a rebuild into the same slot. The old meshes stay drawn meanwhile.
    pub fn invalidate_all(&mut self) {
        for slot in &mut self.slots {
            if slot.retired {
                continue;
            }
            match slot.state {
                SlotState::Built => slot.state = SlotState::Stale,
                SlotState::Pending => slot.dirty = true,
                SlotState::Stale | SlotState::Free => {}
            }
        }
    }

    // The build changed in the same generation: every slot with a drawn mesh is retired, the rest
    // are freed, and every wanted key gets a new slot. `shift` is the shift the slots were drawn
    // with; every retired slot keeps drawing at that place while the next plans draw the new build
    // without it.
    pub fn retire_all(&mut self, shift: [i32; 3]) {
        self.next_change();
        if let Some(job) = self.carry.take() {
            self.spare.push(job);
        }
        for id in 0..self.capacity {
            let slot = self.slots[id as usize];
            if slot.state == SlotState::Free || slot.retired {
                continue;
            }
            if slot.has_mesh && slot.drawn.is_some() {
                self.retire(id);
            } else {
                if slot.unreported() {
                    self.unreported -= 1;
                }
                self.release(id);
                self.pending_frees.push(id);
            }
        }
        if shift == [0; 3] {
            return;
        }
        for slot in &mut self.slots {
            if let (true, Some(draw)) = (slot.retired, slot.drawn.as_mut()) {
                draw.origin = Self::shifted(draw.origin, shift);
            }
        }
    }

    fn shifted(origin: [i32; 3], shift: [i32; 3]) -> [i32; 3] {
        [0, 1, 2].map(|axis| origin[axis].wrapping_add(shift[axis]))
    }

    // The view changed: the next plan retires the slots whose key is no longer wanted.
    pub fn hold_unwanted(&mut self) {
        self.next_change();
        self.hold_unwanted = true;
    }

    // A change while slots are retired keeps the count of the change that retired them, so every
    // slot made since then stays hidden until the swap.
    fn next_change(&mut self) {
        if self.retired == 0 {
            self.change = self.change.wrapping_add(1);
        }
    }

    // False while the wanted list the caller passes is not yet complete, as while a build is
    // scanned. The retired slots are kept until it is.
    pub fn set_wanted_complete(&mut self, complete: bool) {
        self.wanted_complete = complete;
    }

    // Slots that hold a mesh of a previous build or view.
    pub fn retired_slots(&self) -> u32 {
        self.retired
    }

    // Fills `frame` in place. `wanted` is nearest first; its first `undrawn_lead` keys are built but
    // not drawn. Every draw is moved by `shift`.
    pub fn plan(
        &mut self,
        build: Option<&GhostBuild>,
        wanted: &[SectionKey],
        groups: &MiniatureGroups,
        shift: [i32; 3],
        frame: &mut GhostFrame,
    ) {
        self.plans = self.plans.wrapping_add(1);
        self.spare.append(&mut frame.jobs);
        frame.draws.clear();
        frame.frees.clear();
        frame.clear_all = false;
        frame.cut = false;
        frame.palette = None;
        frame.slot_capacity = self.capacity;

        let revision = build.map(|build| build.revision);
        if revision != self.generation {
            self.reset();
            self.generation = revision;
            frame.clear_all = true;
            frame.palette = build.map(|build| Arc::new(build.palette.clone()));
        }
        frame.generation = revision.unwrap_or(0);
        let Some(build) = build else {
            return;
        };

        frame.frees.append(&mut self.pending_frees);
        for key in wanted {
            if let Some(&id) = self.by_key.get(key) {
                self.slots[id as usize].seen = true;
            }
        }
        self.free_unwanted(frame);
        self.hold_unwanted = false;
        self.hand_out_jobs(build, wanted, groups, frame);
        if self.retired > 0 && self.wanted_complete && self.all_built(wanted) {
            self.release_retired(frame);
        }
        let holding = self.retired > 0;
        if holding {
            for slot in &self.slots {
                if let (true, Some(draw)) = (slot.retired, slot.drawn) {
                    frame.draws.push(GhostDraw { origin: Self::shifted(draw.origin, shift), ..draw });
                }
            }
        }

        for key in wanted.iter().take(self.undrawn_lead) {
            if let Some(&id) = self.by_key.get(key) {
                self.slots[id as usize].drawn = None;
            }
        }
        for key in wanted.iter().skip(self.undrawn_lead) {
            let Some(&id) = self.by_key.get(key) else {
                continue;
            };
            let slot = &mut self.slots[id as usize];
            if !slot.has_mesh || (holding && slot.change == self.change) {
                continue;
            }
            let draw = GhostSections::draw(build, *key, id, slot.origin);
            slot.drawn = Some(draw);
            frame.draws.push(GhostDraw { origin: Self::shifted(draw.origin, shift), ..draw });
        }
    }

    // True when every wanted key that has not failed holds a mesh.
    fn all_built(&self, wanted: &[SectionKey]) -> bool {
        wanted.iter().all(|key| match self.by_key.get(key) {
            Some(&id) => self.slots[id as usize].has_mesh,
            None => self.failed.contains(key),
        })
    }

    fn retire(&mut self, id: u32) {
        let slot = &mut self.slots[id as usize];
        if let Some(key) = slot.key.take() {
            self.by_key.remove(&key);
        }
        slot.retired = true;
        slot.seen = false;
        self.retired += 1;
        if self.building == Some(id) {
            self.building = None;
        }
    }

    // Frees one retired slot. False when there is none.
    fn release_one_retired(&mut self, frame: &mut GhostFrame) -> bool {
        let Some(id) = (0..self.capacity).find(|id| self.slots[*id as usize].retired) else {
            return false;
        };
        self.release(id);
        frame.frees.push(id);
        true
    }

    fn release_retired(&mut self, frame: &mut GhostFrame) {
        for id in 0..self.capacity {
            if self.slots[id as usize].retired {
                self.release(id);
                frame.frees.push(id);
            }
        }
    }

    // Records the shim's answers. A report of another generation is dropped.
    pub fn apply_report(&mut self, report: &GhostReport) {
        for built in &report.built {
            if Some(built.generation) != self.generation {
                continue;
            }
            let Some(slot) = self.slots.get_mut(built.slot as usize) else {
                continue;
            };
            if slot.state != SlotState::Pending || slot.serial != built.serial {
                continue;
            }
            if !slot.reported {
                slot.reported = true;
                self.unreported -= 1;
            }
            if built.ok && slot.is_group() && !slot.finishing {
                slot.answered = true;
            } else if built.ok {
                if self.building == Some(built.slot) {
                    self.building = None;
                }
                slot.has_mesh = true;
                self.used_bytes = self.used_bytes - slot.bytes + built.bytes;
                slot.bytes = built.bytes;
                slot.state = if slot.dirty { SlotState::Stale } else { SlotState::Built };
                slot.dirty = false;
            } else {
                if let Some(key) = slot.key
                    && self.failed.len() < Self::MAX_FAILED
                {
                    self.failed.insert(key);
                }
                self.release(built.slot);
                self.pending_frees.push(built.slot);
            }
        }
    }

    fn reset(&mut self) {
        self.slots.fill(Slot::new());
        self.used_bytes = 0;
        if let Some(job) = self.carry.take() {
            self.spare.push(job);
        }
        self.by_key.clear();
        self.free.clear();
        self.free.extend((0..self.capacity).rev());
        self.pending_frees.clear();
        self.failed.clear();
        self.unreported = 0;
        self.retired = 0;
        self.hold_unwanted = false;
        self.building = None;
    }

    fn release(&mut self, id: u32) {
        if let Some(key) = self.slots[id as usize].key {
            self.by_key.remove(&key);
        }
        if self.slots[id as usize].retired {
            self.retired -= 1;
        }
        if self.building == Some(id) {
            self.building = None;
        }
        self.used_bytes -= self.slots[id as usize].bytes;
        self.slots[id as usize] = Slot::new();
        self.free.push(id);
    }

    // Frees slots whose key left the wanted list, or retires them when the view just changed. A
    // late report for a freed pending one carries a serial that no slot holds any more, so it is
    // dropped.
    fn free_unwanted(&mut self, frame: &mut GhostFrame) {
        for id in 0..self.slots.len() {
            let slot = &mut self.slots[id];
            let seen = slot.seen;
            slot.seen = false;
            if slot.retired {
                continue;
            }
            let unreported = slot.unreported();
            if !seen && self.hold_unwanted && slot.has_mesh && slot.drawn.is_some() {
                self.retire(id as u32);
                continue;
            }
            if !seen && slot.state != SlotState::Free {
                if unreported {
                    self.unreported -= 1;
                }
                self.release(id as u32);
                frame.frees.push(id as u32);
            }
        }
    }

    // A new job finds no free slot, or no room in the budget.
    fn needs_room(&self) -> bool {
        !(self.free.last().is_some() && self.fits_budget())
    }

    // Freeing slots can make room: the reserve for the jobs without a report fits the budget.
    fn room_can_be_made(&self) -> bool {
        (u64::from(self.unreported) + 1) * Self::SLOT_BYTES_ESTIMATE <= self.budget_bytes
    }

    fn fits_budget(&self) -> bool {
        let reserved = (u64::from(self.unreported) + 1) * Self::SLOT_BYTES_ESTIMATE;
        self.used_bytes + reserved <= self.budget_bytes
    }

    // The slot of the farthest wanted key below `cursor` and after `index` that holds a finished
    // mesh. Moves `cursor` down past the keys it looked at.
    fn find_victim(&self, wanted: &[SectionKey], index: usize, cursor: &mut usize) -> Option<u32> {
        while *cursor > index + 1 {
            *cursor -= 1;
            let Some(&id) = self.by_key.get(&wanted[*cursor]) else {
                continue;
            };
            if matches!(self.slots[id as usize].state, SlotState::Built | SlotState::Stale) {
                return Some(id);
            }
        }
        None
    }

    fn hand_out_jobs(
        &mut self,
        build: &GhostBuild,
        wanted: &[SectionKey],
        groups: &MiniatureGroups,
        frame: &mut GhostFrame,
    ) {
        let mut spent = 0_u32;
        let mut cursor = wanted.len();
        for (index, key) in wanted.iter().enumerate() {
            let group = key.part == SectionPart::MiniatureGroup;
            let members = if group { groups.members(key.x as u32) } else { &[] };
            if group && members.is_empty() {
                continue;
            }
            let existing = self.by_key.get(key).copied();
            let mut step = (0, true);
            let rebuild = match existing {
                Some(id) => {
                    let slot = &self.slots[id as usize];
                    let expired = self.plans.wrapping_sub(slot.issued_at) >= Self::PENDING_PLANS;
                    if group {
                        match slot.group_step(expired, self.building.is_none()) {
                            Some(next) => step = next,
                            None => continue,
                        }
                        true
                    } else {
                        match slot.state {
                            SlotState::Stale => true,
                            SlotState::Pending if expired => true,
                            _ => continue,
                        }
                    }
                }
                None if self.failed.contains(key) => continue,
                None if group && self.building.is_some() => continue,
                None => false,
            };
            if step.0 as usize > members.len() {
                step = (0, true);
            }

            while !rebuild && self.needs_room() && self.room_can_be_made() && self.release_one_retired(frame) {}
            let mut victim = None;
            if !rebuild && self.needs_room() {
                victim = self.find_victim(wanted, index, &mut cursor);
                if victim.is_none() {
                    frame.cut = true;
                    break;
                }
            }

            let mut job = match self.carry.take() {
                Some(carried) if !group && carried.key == *key => carried,
                other => {
                    if let Some(stale) = other {
                        self.spare.push(stale);
                    }
                    let mut fresh = self.spare.pop().unwrap_or_else(|| GhostJob::new(*key));
                    match members.get(step.0 as usize) {
                        Some(member) => GhostSections::fill_member_job(build, *member, &mut fresh),
                        None if group => GhostSections::fill_finish_job(build, *key, &mut fresh),
                        None => GhostSections::fill_job(build, *key, &mut fresh),
                    }
                    fresh.key = *key;
                    fresh
                }
            };
            let cells = Self::interior_cells(&job).max(1);
            if !frame.jobs.is_empty() && spent.saturating_add(cells) > self.cells_per_frame {
                self.carry = Some(job);
                break;
            }
            spent = spent.saturating_add(cells);

            let id = match (existing, victim) {
                (Some(id), _) => id,
                (None, Some(id)) => {
                    self.release(id);
                    self.free.pop();
                    frame.cut = true;
                    id
                }
                (None, None) => match self.free.pop() {
                    Some(id) => id,
                    None => {
                        self.carry = Some(job);
                        break;
                    }
                },
            };
            let slot = &mut self.slots[id as usize];
            if !rebuild {
                *slot = Slot::new();
                slot.change = self.change;
                self.unreported += 1;
            }
            slot.key = Some(*key);
            slot.state = SlotState::Pending;
            slot.origin = job.origin;
            slot.dirty = false;
            slot.serial = self.next_serial;
            slot.issued_at = self.plans;
            self.by_key.insert(*key, id);
            job.slot = id;
            job.serial = self.next_serial;
            if group {
                if step.1 {
                    slot.group_serial = job.serial;
                    slot.group_expiries = 0;
                } else if !slot.answered && step.0 + 1 == slot.group_next {
                    slot.group_expiries += 1;
                } else {
                    slot.group_expiries = 0;
                }
                job.group_serial = slot.group_serial;
                job.group_index = step.0;
                slot.group_next = step.0 + 1;
                slot.answered = false;
                slot.finishing = job.mode == GhostJobMode::Finish;
                self.building = Some(id);
            }
            self.next_serial = self.next_serial.wrapping_add(1);
            frame.jobs.push(job);
        }
    }

    // Filled cells inside the part, padding excluded.
    fn interior_cells(job: &GhostJob) -> u32 {
        let [dx, dy, dz] = job.dims.map(|n| n as usize);
        let mut total = 0;
        for y in 1..=dy {
            for z in 1..=dz {
                let row = (y * (dz + 2) + z) * (dx + 2);
                total += job.cells[row + 1..row + 1 + dx].iter().filter(|cell| **cell != GhostPalette::NONE).count();
            }
        }
        total as u32
    }
}
