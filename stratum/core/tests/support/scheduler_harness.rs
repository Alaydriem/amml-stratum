use stratum_core::config::GhostConfig;
use stratum_core::services::GhostScheduler;
use stratum_core::services::GhostSections;
use stratum_core::structs::ghost::{
    GhostBuild, GhostFrame, GhostJob, GhostReport, GhostView, MiniatureGroups, SectionKey, SectionPart, SlotBuilt,
};

use super::{GhostFixture, SectionAssert};

// Drives a scheduler the way the ghost service and the shim do.
pub struct SchedulerHarness {
    pub scheduler: GhostScheduler,
    pub frame: GhostFrame,
    pub groups: MiniatureGroups,
}

impl SchedulerHarness {
    pub fn new(mesh_budget_mb: u32, cells_per_frame: u32) -> Self {
        let config = GhostConfig { mesh_budget_mb, cells_per_frame, ..GhostConfig::default() };
        Self { scheduler: GhostScheduler::new(&config), frame: GhostFrame::new(), groups: MiniatureGroups::new() }
    }

    // `count` single-cell sections in a row along X, 16 cells apart.
    pub fn row(count: i32) -> GhostBuild {
        let boxes: Vec<([i32; 3], [i32; 3])> = (0..count).map(|n| ([n * 16, 0, 0], [n * 16, 0, 0])).collect();
        GhostFixture::blocks(&boxes)
    }

    // Wanted keys nearest to the origin first.
    pub fn wanted(build: &GhostBuild) -> Vec<SectionKey> {
        SectionAssert::wanted(build, GhostView::Whole, [0, 0, 0], 100)
    }

    // Wanted keys within `distance` chunks of the origin, nearest first.
    pub fn wanted_within(build: &GhostBuild, distance: u32) -> Vec<SectionKey> {
        let mut occupied = Vec::new();
        GhostSections::occupied(build, 600, &mut occupied, &mut Vec::new());
        let mut out = Vec::new();
        GhostSections::wanted(&occupied, GhostView::Whole, [0, 0, 0], distance, &mut out);
        out
    }

    // Every occupied key of `build`.
    pub fn occupied(build: &GhostBuild) -> Vec<SectionKey> {
        let mut occupied = Vec::new();
        GhostSections::occupied(build, 600, &mut occupied, &mut Vec::new());
        occupied
    }

    // Cuts the occupied keys of `build` into groups of `sizes` members each.
    pub fn cut_groups(&mut self, build: &GhostBuild, sizes: &[usize]) {
        let occupied = Self::occupied(build);
        let mut keys = Vec::new();
        let mut cells = Vec::new();
        let mut next = occupied.iter();
        for size in sizes {
            for member in 0..*size {
                keys.push(*next.next().unwrap());
                cells.push(if member == 0 { 10_000 } else { 1 });
            }
        }
        self.groups.cut(&keys, &cells);
    }

    // The key of each group.
    pub fn group_keys(&self) -> Vec<SectionKey> {
        self.groups.keys().collect()
    }

    // Plans and reports every job as built with `bytes` each, until a plan hands out nothing. The
    // frame then holds that last plan.
    pub fn settle(&mut self, build: &GhostBuild, wanted: &[SectionKey], bytes: u64) {
        for _ in 0..64 {
            self.plan(build, wanted);
            if self.frame.jobs.is_empty() {
                return;
            }
            self.report_jobs(true, bytes);
        }
        panic!("the scheduler did not settle");
    }

    // The slot of each job in the current frame, with its key.
    pub fn job_slots(&self) -> Vec<(SectionKey, u32)> {
        self.frame.jobs.iter().map(|job| (job.key, job.slot)).collect()
    }

    pub fn plan(&mut self, build: &GhostBuild, wanted: &[SectionKey]) {
        self.plan_shifted(build, wanted, [0, 0, 0]);
    }

    // Plans with every draw moved by `shift`.
    pub fn plan_shifted(&mut self, build: &GhostBuild, wanted: &[SectionKey], shift: [i32; 3]) {
        self.scheduler.plan(Some(build), wanted, &self.groups, shift, &mut self.frame);
    }

    // Reports every job of the current frame back with `bytes` each.
    pub fn report_jobs(&mut self, ok: bool, bytes: u64) {
        self.report_matching(|_| true, ok, bytes);
    }

    // Reports the jobs of the current frame that `keep` accepts back with `bytes` each.
    pub fn report_matching(&mut self, keep: impl Fn(&GhostJob) -> bool, ok: bool, bytes: u64) {
        let mut report = GhostReport::new();
        for job in self.frame.jobs.iter().filter(|job| keep(job)) {
            report.built.push(SlotBuilt {
                slot: job.slot,
                serial: job.serial,
                generation: self.frame.generation,
                bytes,
                ok,
            });
        }
        self.scheduler.apply_report(&report);
    }

    // The jobs of the current frame that build a mini ghost group.
    pub fn group_jobs(&self) -> Vec<&GhostJob> {
        self.frame.jobs.iter().filter(|job| job.key.part == SectionPart::MiniatureGroup).collect()
    }

    pub fn job_keys(&self) -> Vec<SectionKey> {
        self.frame.jobs.iter().map(|job| job.key).collect()
    }

    pub fn draw_slots(&self) -> Vec<u32> {
        self.frame.draws.iter().map(|draw| draw.slot).collect()
    }
}
