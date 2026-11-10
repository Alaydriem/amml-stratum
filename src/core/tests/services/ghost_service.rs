use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use stratum_core::config::GhostConfig;
use stratum_core::services::{GhostSections, GhostService};
use stratum_core::structs::ghost::{
    GhostDraw, GhostFrame, GhostJob, GhostJobMode, GhostPalette, GhostReport, MiniatureGroups, SectionKey, SectionPart,
    SlotBuilt,
};
use stratum_core::structs::library::{Library, StructureId};
use stratum_core::structs::marker::{Marker, MarkerId, MarkerTarget, OpacityStep, Rotation, ViewMode};
use stratum_core::structs::structure::{BlockLayer, BlockState, BlockStates, SparseLayer, Structure, StructureSize};

use crate::support::{Frames, GhostFixture};

struct Rig {
    service: GhostService,
    frame: GhostFrame,
    library: Library,
    // The steady clock and the player's feet of every update.
    time: f64,
    feet: [i32; 3],
    // The key of the latest job handed out for each slot.
    slots: HashMap<u32, SectionKey>,
    // Every mini ghost group job handed out: its key, mode, group serial, group index and group offset.
    group_jobs: Vec<(SectionKey, GhostJobMode, u32, u32, [i32; 3])>,
}

impl Rig {
    fn new() -> Self {
        Self::with_config(GhostConfig::default())
    }

    fn with_config(config: GhostConfig) -> Self {
        Self {
            service: GhostService::new(config),
            frame: GhostFrame::new(),
            library: Library::new(),
            time: 1.0,
            feet: [0, 64, 0],
            slots: HashMap::new(),
            group_jobs: Vec::new(),
        }
    }

    // A rig whose build is the full ghost only: no mini ghost, and the marker is the only extra part.
    fn full() -> Self {
        Self::with_config(GhostConfig { miniature: false, ..GhostConfig::default() })
    }

    // A marker that draws the whole build.
    fn marker(id: u64, rotation: Rotation) -> Marker {
        let mut marker = Marker {
            id: MarkerId(id),
            rotation,
            ..GhostFixture::marker([0, 64, 0], MarkerTarget::Structure(StructureId(7)))
        };
        marker.view.view_mode = ViewMode::Whole;
        marker
    }

    // A marker that draws layer `layer` of the build.
    fn layer_marker(layer: u32) -> Marker {
        let mut marker = Self::marker(1, Rotation::R0);
        marker.view.view_mode = ViewMode::Layer;
        marker.view.layer = layer;
        marker
    }

    // True for the parts of the full ghost: not the mini ghost and not the marker.
    fn is_full(key: &SectionKey) -> bool {
        !key.part.is_miniature() && key.part != SectionPart::Marker
    }

    // The full ghost keys of a set of keys.
    fn full_keys(keys: &HashSet<SectionKey>) -> Vec<SectionKey> {
        keys.iter().copied().filter(Self::is_full).collect()
    }

    // The jobs of the current frame that build full ghost sections.
    fn full_jobs(&self) -> Vec<&GhostJob> {
        self.frame.jobs.iter().filter(|job| Self::is_full(&job.key)).collect()
    }

    // The draws of the current frame whose slot was last built for a full ghost section.
    fn full_draws(&self) -> Vec<GhostDraw> {
        self.draws_of(|key| Self::is_full(&key))
    }

    // The draws of the current frame whose slot was last built for a mini ghost group.
    fn miniature_draws(&self) -> Vec<GhostDraw> {
        self.draws_of(|key| key.part.is_miniature())
    }

    // The draws of the current frame whose slot was last built for the structure block marker.
    fn marker_draws(&self) -> Vec<GhostDraw> {
        self.draws_of(|key| key.part == SectionPart::Marker)
    }

    fn draws_of(&self, wanted: impl Fn(SectionKey) -> bool) -> Vec<GhostDraw> {
        self.frame
            .draws
            .iter()
            .filter(|draw| self.slots.get(&draw.slot).is_some_and(|key| wanted(*key)))
            .copied()
            .collect()
    }

    // Updates until the turn the worker runs for `marker` is in the frame.
    fn await_turn(&mut self, marker: &Marker, loaded: &[(u64, Arc<Structure>)]) {
        let start = Instant::now();
        loop {
            self.update(Some(marker), loaded);
            if !self.service.turning() && self.frame.rotation == marker.rotation {
                return;
            }
            assert!(start.elapsed() < Duration::from_secs(60), "the turn did not arrive");
            std::thread::sleep(Duration::from_millis(1));
        }
    }

    // `draws`, each moved by `delta`.
    fn moved_draws(draws: &[GhostDraw], delta: [i32; 3]) -> Vec<GhostDraw> {
        draws
            .iter()
            .map(|draw| GhostDraw { origin: [0, 1, 2].map(|axis| draw.origin[axis] + delta[axis]), ..*draw })
            .collect()
    }

    fn update(&mut self, marker: Option<&Marker>, loaded: &[(u64, Arc<Structure>)]) {
        self.update_in("W1", marker, loaded);
    }

    fn update_in(&mut self, world: &str, marker: Option<&Marker>, loaded: &[(u64, Arc<Structure>)]) {
        let loaded = GhostFixture::loaded(loaded);
        let status = Frames::at(world, self.feet, self.time);
        self.service.update(marker, &self.library, &loaded, &status, true, &mut self.frame);
        self.record_jobs();
    }

    fn record_jobs(&mut self) {
        self.slots.extend(self.frame.jobs.iter().map(|job| (job.slot, job.key)));
        let groups = self.frame.jobs.iter().filter(|job| job.key.part == SectionPart::MiniatureGroup);
        self.group_jobs
            .extend(groups.map(|job| (job.key, job.mode, job.group_serial, job.group_index, job.group_offset)));
    }

    // The group offsets of the Append jobs of the latest build of each group in `jobs`, sorted.
    fn member_offsets(jobs: &[(SectionKey, GhostJobMode, u32, u32, [i32; 3])]) -> Vec<[i32; 3]> {
        let mut latest: HashMap<SectionKey, usize> = HashMap::new();
        for (index, job) in jobs.iter().enumerate() {
            if job.3 == 0 {
                latest.insert(job.0, index);
            }
        }
        let mut offsets: Vec<[i32; 3]> = jobs
            .iter()
            .enumerate()
            .filter(|(index, job)| {
                job.1 == GhostJobMode::Append && latest.get(&job.0).is_some_and(|start| index >= start)
            })
            .map(|(_, job)| job.4)
            .collect();
        offsets.sort();
        offsets
    }

    // The group offset of each full key, for a build with its lowest corner at (0, 64, 0).
    fn key_offsets(keys: &[SectionKey]) -> Vec<[i32; 3]> {
        let mut offsets: Vec<[i32; 3]> = keys
            .iter()
            .map(|key| {
                [
                    key.x * SectionKey::SIZE,
                    key.y * SectionKey::SIZE + key.part.y_offset() - 64,
                    key.z * SectionKey::SIZE,
                ]
            })
            .collect();
        offsets.sort();
        offsets
    }

    // The ghost is not allowed to draw, as when the marker is hidden.
    fn update_hidden(&mut self, marker: &Marker, loaded: &[(u64, Arc<Structure>)]) {
        let loaded = GhostFixture::loaded(loaded);
        let status = Frames::at("W1", self.feet, self.time);
        self.service.update(Some(marker), &self.library, &loaded, &status, false, &mut self.frame);
        self.record_jobs();
    }

    fn built_report(&self) -> GhostReport {
        let mut report = GhostReport::new();
        for job in &self.frame.jobs {
            report.built.push(SlotBuilt {
                slot: job.slot,
                serial: job.serial,
                generation: self.frame.generation,
                bytes: 64,
                ok: true,
            });
        }
        report
    }

    // Answers every job of the current frame as built.
    fn answer_jobs(&mut self) {
        let report = self.built_report();
        self.service.apply_report(&report);
    }

    // Ticks until the scan is done and no job is left, answering every job. Returns the keys that
    // got a job, the slots freed and whether any frame cleared everything.
    fn run_to_rest(&mut self, marker: &Marker, loaded: &[(u64, Arc<Structure>)]) -> (HashSet<SectionKey>, usize, bool) {
        let mut keys = HashSet::new();
        let mut frees = 0;
        let mut cleared = false;
        for _ in 0..400 {
            self.update(Some(marker), loaded);
            keys.extend(self.frame.jobs.iter().map(|job| job.key));
            frees += self.frame.frees.len();
            cleared |= self.frame.clear_all;
            self.answer_jobs();
            if self.service.turning() {
                std::thread::sleep(Duration::from_millis(1));
                continue;
            }
            if !self.service.scanning() && self.frame.jobs.is_empty() {
                return (keys, frees, cleared);
            }
        }
        panic!("the ghost did not settle");
    }
}

#[test]
fn the_structure_target_of_the_marker_is_what_the_controller_must_load() {
    let mut rig = Rig::new();
    let marker = Rig::marker(1, Rotation::R0);

    rig.update(Some(&marker), &[]);

    assert_eq!(rig.service.needed(), [StructureId(7)]);
}

#[test]
fn a_library_edit_that_leaves_the_layout_alone_keeps_the_generation() {
    let mut rig = Rig::new();
    let marker = Rig::marker(1, Rotation::R0);
    let loaded = [(7, GhostFixture::structure([2, 1, 1], &["stone", "stone"]))];

    rig.update(Some(&marker), &[]);
    rig.update(Some(&marker), &loaded);
    let generation = rig.frame.generation;
    assert!(rig.frame.palette.is_some());

    rig.service.library_changed();
    rig.update(Some(&marker), &loaded);

    assert_eq!(rig.frame.generation, generation);
    assert!(rig.frame.palette.is_none());
    assert!(!rig.frame.clear_all);
}

#[test]
fn a_different_marker_starts_a_new_generation_and_clears_the_old_slots() {
    let mut rig = Rig::new();
    let first = Rig::marker(1, Rotation::R0);
    let second = Rig::marker(2, Rotation::R0);
    let loaded = [(7, GhostFixture::structure([2, 1, 1], &["stone", "stone"]))];

    rig.update(Some(&first), &[]);
    rig.update(Some(&first), &loaded);
    let generation = rig.frame.generation;

    rig.update(Some(&second), &loaded);

    assert!(rig.frame.clear_all);
    assert_ne!(rig.frame.generation, generation);
}

#[test]
fn a_new_frame_allowance_scans_the_build_again_with_the_new_split() {
    let mut rig = Rig::full();
    let marker = Rig::marker(1, Rotation::R0);
    let loaded = [(7, GhostFixture::structure([16, 16, 16], &["stone"; 4096]))];
    rig.update(Some(&marker), &[]);
    let mut halves = Vec::new();
    for _ in 0..4 {
        rig.update(Some(&marker), &loaded);
        halves = rig.full_jobs().iter().map(|job| job.key.part).collect();
        if !halves.is_empty() {
            break;
        }
        rig.answer_jobs();
    }
    assert_eq!(halves, [SectionPart::Lower]);

    rig.service.reconfigure(GhostConfig { cells_per_frame: 2048, miniature: false, ..GhostConfig::default() });
    rig.update(Some(&marker), &loaded);

    let parts: Vec<SectionPart> = rig.full_jobs().iter().map(|job| job.key.part).collect();
    assert_eq!(parts, [SectionPart::Whole]);
    assert_eq!(rig.frame.frees.len(), 1);
}

#[test]
fn the_mesh_budget_waits_for_a_restart() {
    let mut rig = Rig::new();
    let capacity = rig.service.slot_capacity();

    rig.service.reconfigure(GhostConfig { mesh_budget_mb: 1, draw_distance: 9, ..GhostConfig::default() });

    assert_eq!(rig.service.slot_capacity(), capacity);
    assert_eq!(rig.service.budget_bytes(), GhostConfig::default().mesh_budget_bytes());
    assert_eq!(rig.service.config().draw_distance, 9);
}

#[test]
fn the_fallback_counts_are_those_of_the_last_resolved_palette() {
    let mut rig = Rig::new();
    let resolved = |unknown: u32, default_state: u32| {
        let mut report = GhostReport::new();
        report.palette_resolved = true;
        report.unknown_blocks = unknown;
        report.default_state_blocks = default_state;
        report
    };

    rig.service.apply_report(&resolved(3, 2));
    rig.service.apply_report(&resolved(3, 2));
    rig.service.apply_report(&GhostReport::new());

    assert_eq!(rig.service.unknown_blocks(), 3);
    assert_eq!(rig.service.default_state_blocks(), 2);
    rig.service.apply_report(&resolved(0, 1));
    assert_eq!(rig.service.unknown_blocks(), 0);
    assert_eq!(rig.service.default_state_blocks(), 1);
}

#[test]
fn a_palette_past_the_limit_is_refused_with_one_notice() {
    let mut rig = Rig::new();
    let marker = Rig::marker(1, Rotation::R0);
    let loaded = [(7, GhostFixture::distinct(GhostPalette::MAX_ENTRIES + 1))];

    rig.update(Some(&marker), &[]);
    rig.update(Some(&marker), &loaded);
    rig.update(Some(&marker), &loaded);

    assert!(rig.service.take_refused_notice());
    assert!(!rig.service.take_refused_notice());
    assert!(rig.frame.draws.is_empty() && rig.frame.jobs.is_empty());
}

#[test]
fn a_big_build_draws_before_its_scan_is_done() {
    let mut rig = Rig::new();
    let marker = Rig::marker(1, Rotation::R0);
    let names = vec!["stone"; 80 * 16 * 80];
    let loaded = [(7, GhostFixture::structure([80, 16, 80], &names))];

    rig.update(Some(&marker), &[]);
    rig.update(Some(&marker), &loaded);

    assert!(rig.service.scanning());
    assert!(rig.frame.palette.is_some());
    assert!(!rig.frame.jobs.is_empty());

    for _ in 0..4 {
        rig.update(Some(&marker), &loaded);
    }
    assert!(!rig.service.scanning());
}

#[test]
fn a_live_frame_allowance_change_keeps_the_built_slots_while_the_scan_runs() {
    let mut rig = Rig::new();
    let marker = Rig::marker(1, Rotation::R0);
    let names = vec!["stone"; 80 * 16 * 80];
    let loaded = [(7, GhostFixture::structure([80, 16, 80], &names))];
    rig.update(Some(&marker), &[]);
    let mut settled = false;
    for _ in 0..400 {
        rig.update(Some(&marker), &loaded);
        let mut report = GhostReport::new();
        for job in &rig.frame.jobs {
            report.built.push(SlotBuilt {
                slot: job.slot,
                serial: job.serial,
                generation: rig.frame.generation,
                bytes: 64,
                ok: true,
            });
        }
        rig.service.apply_report(&report);
        if !rig.service.scanning() && rig.frame.jobs.is_empty() {
            settled = true;
            break;
        }
    }
    assert!(settled);
    let live = rig.service.live_slots();
    let drawn = rig.frame.draws.len();
    assert!(live > 16);

    rig.service.reconfigure(GhostConfig { cells_per_frame: 700, ..GhostConfig::default() });
    rig.update(Some(&marker), &loaded);

    assert!(rig.service.scanning());
    assert!(rig.frame.frees.is_empty());
    assert!(!rig.frame.clear_all);
    assert_eq!(rig.service.live_slots(), live);
    assert_eq!(rig.frame.draws.len(), drawn);
}

#[test]
fn a_world_change_gives_the_next_build_a_new_generation_with_palette_and_clear() {
    let mut rig = Rig::new();
    let marker = Rig::marker(1, Rotation::R0);
    let loaded = [(7, GhostFixture::structure([2, 1, 1], &["stone", "stone"]))];
    rig.update(Some(&marker), &[]);
    rig.update(Some(&marker), &loaded);
    rig.answer_jobs();
    rig.update(Some(&marker), &loaded);
    let generation = rig.frame.generation;
    let epoch = rig.frame.world_epoch;

    rig.update_in("W2", None, &[]);
    assert!(rig.frame.world_epoch > epoch);
    rig.update_in("W2", Some(&marker), &[]);
    rig.update_in("W2", Some(&marker), &loaded);

    assert!(rig.frame.generation > generation);
    assert!(rig.frame.clear_all);
    assert!(rig.frame.palette.is_some());
}

#[test]
fn a_finished_rescan_with_the_same_split_swaps_in_the_keys_and_keeps_the_built_slots() {
    let mut rig = Rig::new();
    let marker = Rig::marker(1, Rotation::R0);
    let names = vec!["stone"; 80 * 16 * 80];
    let loaded = [(7, GhostFixture::structure([80, 16, 80], &names))];
    rig.update(Some(&marker), &[]);
    rig.run_to_rest(&marker, &loaded);
    let live = rig.service.live_slots();
    let drawn = rig.frame.draws.len();

    rig.service.reconfigure(GhostConfig { cells_per_frame: 700, ..GhostConfig::default() });
    let (keys, frees, cleared) = rig.run_to_rest(&marker, &loaded);

    assert!(keys.is_empty());
    assert_eq!(frees, 0);
    assert!(!cleared);
    assert!(!rig.service.scanning());
    assert_eq!(rig.service.live_slots(), live);
    assert_eq!(rig.frame.draws.len(), drawn);
}

#[test]
fn a_finished_rescan_with_a_new_split_builds_only_the_new_keys() {
    let mut rig = Rig::new();
    let marker = Rig::marker(1, Rotation::R0);
    let names = vec!["stone"; 80 * 16 * 80];
    let loaded = [(7, GhostFixture::structure([80, 16, 80], &names))];
    rig.update(Some(&marker), &[]);
    let (before, _, _) = rig.run_to_rest(&marker, &loaded);

    rig.service.reconfigure(GhostConfig { cells_per_frame: 2048, ..GhostConfig::default() });
    let (after, frees, cleared) = rig.run_to_rest(&marker, &loaded);

    let before: HashSet<SectionKey> = Rig::full_keys(&before).into_iter().collect();
    let after: HashSet<SectionKey> = Rig::full_keys(&after).into_iter().collect();
    assert!(!after.is_empty());
    assert!(after.is_disjoint(&before));
    assert!(frees > 0);
    assert!(!cleared);
    assert_eq!(rig.service.live_slots() as usize, rig.frame.draws.len());
}

#[test]
fn a_second_allowance_change_during_a_rescan_ends_with_the_newest_split() {
    let names = vec!["stone"; 80 * 16 * 80];
    let loaded = [(7, GhostFixture::structure([80, 16, 80], &names))];
    let marker = Rig::marker(1, Rotation::R0);

    let mut direct = Rig::new();
    direct.update(Some(&marker), &[]);
    direct.run_to_rest(&marker, &loaded);
    direct.service.reconfigure(GhostConfig { cells_per_frame: 2048, ..GhostConfig::default() });
    direct.run_to_rest(&marker, &loaded);

    let mut twice = Rig::new();
    twice.update(Some(&marker), &[]);
    twice.run_to_rest(&marker, &loaded);
    twice.service.reconfigure(GhostConfig { cells_per_frame: 700, ..GhostConfig::default() });
    twice.update(Some(&marker), &loaded);
    assert!(twice.service.scanning());
    twice.service.reconfigure(GhostConfig { cells_per_frame: 2048, ..GhostConfig::default() });
    let (_, _, cleared) = twice.run_to_rest(&marker, &loaded);

    assert!(!cleared);
    assert!(!twice.service.scanning());
    assert_eq!(twice.service.live_slots() as usize, twice.frame.draws.len());
    assert_eq!(twice.service.live_slots(), direct.service.live_slots());
}

#[test]
fn the_full_ghost_jobs_skip_matching_blocks() {
    let mut rig = Rig::new();
    let marker = Rig::marker(1, Rotation::R0);
    let loaded = [(7, GhostFixture::structure([4, 4, 4], &["stone"; 64]))];
    rig.update(Some(&marker), &[]);

    rig.update(Some(&marker), &loaded);

    assert!(!rig.full_jobs().is_empty());
    assert!(rig.full_jobs().iter().all(|job| job.skip_matching));
}

#[test]
fn marker_layer_view_draws_one_layer() {
    let mut rig = Rig::new();
    let marker = Rig::layer_marker(1);
    let loaded = [(7, GhostFixture::structure([2, 3, 2], &["stone"; 12]))];
    rig.update(Some(&marker), &[]);

    let (keys, _, _) = rig.run_to_rest(&marker, &loaded);

    let full = Rig::full_keys(&keys);
    assert!(!full.is_empty());
    assert!(full.iter().all(|key| key.layer == Some(65)));
    assert!(!rig.full_draws().is_empty());
}

#[test]
fn whole_view_draws_every_layer() {
    let mut rig = Rig::new();
    let marker = Rig::marker(1, Rotation::R0);
    let loaded = [(7, GhostFixture::structure([2, 3, 2], &["stone"; 12]))];
    rig.update(Some(&marker), &[]);

    let (keys, _, _) = rig.run_to_rest(&marker, &loaded);

    let full = Rig::full_keys(&keys);
    assert!(!full.is_empty());
    assert!(full.iter().all(|key| key.layer.is_none()));
}

#[test]
fn a_layer_past_the_top_draws_the_whole_build() {
    let mut rig = Rig::new();
    let marker = Rig::layer_marker(3);
    let loaded = [(7, GhostFixture::structure([2, 3, 2], &["stone"; 12]))];
    rig.update(Some(&marker), &[]);

    let (keys, _, _) = rig.run_to_rest(&marker, &loaded);

    let full = Rig::full_keys(&keys);
    assert!(!full.is_empty());
    assert!(full.iter().all(|key| key.layer.is_none()));
}

#[test]
fn frame_opacity_follows_the_marker_step() {
    let mut rig = Rig::new();
    let mut marker = Rig::marker(1, Rotation::R0);
    marker.view.opacity_step = 5;
    let loaded = [(7, GhostFixture::structure([2, 1, 1], &["stone", "stone"]))];

    rig.update(Some(&marker), &[]);
    assert_eq!(rig.frame.opacity, OpacityStep::new(5).alpha());
    rig.update(Some(&marker), &loaded);
    assert_eq!(rig.frame.opacity, 0.80);
    rig.update_hidden(&marker, &loaded);
    assert_eq!(rig.frame.opacity, 0.80);

    marker.view.opacity_step = 0;
    rig.update(Some(&marker), &loaded);
    assert_eq!(rig.frame.opacity, 0.30);
}

#[test]
fn an_opacity_change_rebuilds_every_mesh_and_keeps_drawing_the_old_ones() {
    let mut rig = Rig::new();
    let mut marker = Rig::marker(1, Rotation::R0);
    let loaded = [(7, GhostFixture::structure([20, 1, 1], &["stone"; 20]))];
    rig.update(Some(&marker), &[]);
    rig.run_to_rest(&marker, &loaded);
    let drawn = rig.full_draws();
    assert_eq!(drawn.len(), 2);

    marker.view.opacity_step = 6;
    rig.update(Some(&marker), &loaded);

    assert_eq!(rig.full_jobs().len(), 2);
    assert_eq!(rig.full_draws(), drawn);
    assert!(rig.frame.frees.is_empty());
    assert!(!rig.frame.clear_all);
}

#[test]
fn frame_carries_the_rotation() {
    let mut rig = Rig::new();
    let marker = Rig::marker(1, Rotation::R270);
    let loaded = [(7, GhostFixture::structure([2, 1, 1], &["stone", "stone"]))];

    rig.update(Some(&marker), &[]);
    rig.await_turn(&marker, &loaded);

    assert_eq!(rig.frame.rotation, Rotation::R270);
    assert!(!rig.frame.jobs.is_empty());
}

#[test]
fn old_meshes_stay_until_the_rebuild_is_ready() {
    let mut rig = Rig::new();
    let marker = Rig::marker(1, Rotation::R0);
    let loaded = [(7, GhostFixture::structure([2, 3, 2], &["stone"; 12]))];
    rig.update(Some(&marker), &[]);
    rig.run_to_rest(&marker, &loaded);
    let drawn = rig.full_draws();
    assert!(!drawn.is_empty());

    let layer = Rig::layer_marker(1);
    rig.update(Some(&layer), &loaded);

    assert!(!rig.frame.jobs.is_empty());
    assert_eq!(rig.full_draws(), drawn);
    assert!(rig.frame.frees.is_empty());
    assert!(!rig.frame.clear_all);

    rig.answer_jobs();
    rig.update(Some(&layer), &loaded);
    let after = rig.full_draws();
    assert!(!after.is_empty());
    assert!(after.iter().all(|draw| !drawn.contains(draw)));
    assert!(!rig.frame.frees.is_empty());
}

#[test]
fn a_moved_build_keeps_its_old_meshes_until_the_new_ones_are_ready() {
    let mut rig = Rig::new();
    let marker = Rig::marker(1, Rotation::R0);
    let loaded = [(7, GhostFixture::structure([2, 3, 2], &["stone"; 12]))];
    rig.update(Some(&marker), &[]);
    rig.run_to_rest(&marker, &loaded);
    let generation = rig.frame.generation;
    let drawn = rig.full_draws();
    let shifted = Rig::moved_draws(&drawn, [1, 0, 0]);

    let moved = Marker { origin: [1, 64, 0], ..marker.clone() };
    rig.update(Some(&moved), &loaded);
    rig.time += GhostService::MOVE_SETTLE_SECONDS;
    rig.update(Some(&moved), &loaded);

    assert!(!rig.frame.jobs.is_empty());
    assert_eq!(rig.full_draws(), shifted);
    assert!(!rig.frame.clear_all);
    assert_eq!(rig.frame.generation, generation);
    rig.answer_jobs();

    let (_, _, cleared) = rig.run_to_rest(&moved, &loaded);
    assert!(!cleared);
    assert_eq!(rig.full_draws().len(), drawn.len());
    assert!(rig.full_draws().iter().all(|draw| !drawn.contains(draw) && !shifted.contains(draw)));
}

#[test]
fn a_move_shifts_the_drawn_meshes_without_a_job() {
    let mut rig = Rig::new();
    let marker = Rig::marker(1, Rotation::R0);
    let loaded = [(7, GhostFixture::structure([2, 3, 2], &["stone"; 12]))];
    rig.update(Some(&marker), &[]);
    rig.run_to_rest(&marker, &loaded);
    let drawn = rig.frame.draws.clone();
    assert!(!rig.full_draws().is_empty());

    let moved = Marker { origin: [1, 64, 0], ..marker.clone() };
    rig.update(Some(&moved), &loaded);

    assert!(rig.frame.jobs.is_empty());
    assert!(rig.frame.frees.is_empty());
    assert!(!rig.frame.clear_all);
    assert_eq!(rig.frame.draws, Rig::moved_draws(&drawn, [1, 0, 0]));
}

#[test]
fn held_moves_shift_by_the_total_and_build_nothing() {
    let mut rig = Rig::new();
    rig.time = 0.0;
    let marker = Rig::marker(1, Rotation::R0);
    let loaded = [(7, GhostFixture::structure([2, 3, 2], &["stone"; 12]))];
    rig.update(Some(&marker), &[]);
    rig.run_to_rest(&marker, &loaded);
    let drawn = rig.frame.draws.clone();

    for (step, time) in [0.0, 0.1, 0.2, 0.4].into_iter().enumerate() {
        rig.time = time;
        let moved = Marker { origin: [step as i32 + 1, 64, 0], ..marker.clone() };
        rig.update(Some(&moved), &loaded);
        assert!(rig.frame.jobs.is_empty());
        assert!(rig.frame.frees.is_empty());
    }
    let moved = Marker { origin: [4, 64, 0], ..marker.clone() };
    rig.time = 0.65;
    rig.update(Some(&moved), &loaded);
    assert!(rig.frame.jobs.is_empty());

    assert_eq!(rig.frame.draws, Rig::moved_draws(&drawn, [4, 0, 0]));
    rig.time = 0.4 + GhostService::MOVE_SETTLE_SECONDS;
    rig.update(Some(&moved), &loaded);
    assert!(!rig.frame.jobs.is_empty());
}

#[test]
fn a_held_key_holds_the_settle_until_it_is_released() {
    let mut rig = Rig::new();
    let marker = Rig::marker(1, Rotation::R0);
    let loaded = [(7, GhostFixture::structure([2, 3, 2], &["stone"; 12]))];
    rig.update(Some(&marker), &[]);
    rig.run_to_rest(&marker, &loaded);
    let moved = Marker { origin: [1, 64, 0], ..marker.clone() };

    rig.service.set_keys_held(true);
    rig.update(Some(&moved), &loaded);
    rig.time = 6.0;
    rig.update(Some(&moved), &loaded);
    assert!(rig.frame.jobs.is_empty());

    rig.service.set_keys_held(false);
    rig.time = 7.0;
    rig.update(Some(&moved), &loaded);
    assert!(rig.frame.jobs.is_empty());
    rig.time = 7.25;
    rig.update(Some(&moved), &loaded);
    assert!(rig.frame.jobs.is_empty());

    rig.time = 7.0 + GhostService::MOVE_SETTLE_SECONDS;
    rig.update(Some(&moved), &loaded);
    assert!(!rig.frame.jobs.is_empty());
}

#[test]
fn a_move_back_within_the_quiet_time_draws_at_the_old_place_with_no_rebuild() {
    let mut rig = Rig::new();
    let marker = Rig::marker(1, Rotation::R0);
    let loaded = [(7, GhostFixture::structure([2, 3, 2], &["stone"; 12]))];
    rig.update(Some(&marker), &[]);
    rig.run_to_rest(&marker, &loaded);
    let drawn = rig.frame.draws.clone();

    let moved = Marker { origin: [1, 64, 0], ..marker.clone() };
    rig.update(Some(&moved), &loaded);
    rig.time += 0.1;
    rig.update(Some(&marker), &loaded);

    assert!(rig.frame.jobs.is_empty());
    assert_eq!(rig.frame.draws, drawn);
    rig.time += 0.5;
    rig.update(Some(&marker), &loaded);
    assert!(rig.frame.jobs.is_empty());
    assert!(rig.frame.frees.is_empty());
    assert_eq!(rig.frame.draws, drawn);
}

#[test]
fn a_block_change_during_a_shift_rebuilds_the_slot_of_the_build_place() {
    let mut rig = Rig::new();
    let marker = Rig::marker(1, Rotation::R0);
    let loaded = [(7, GhostFixture::structure([20, 1, 1], &["stone"; 20]))];
    rig.update(Some(&marker), &[]);
    rig.run_to_rest(&marker, &loaded);
    let shifted = Rig::moved_draws(&rig.full_draws(), [1, 0, 0]);
    let moved = Marker { origin: [1, 64, 0], ..marker.clone() };
    rig.update(Some(&moved), &loaded);
    let slot = rig.full_draws().iter().find(|draw| draw.origin == [17, 64, 0]).map(|draw| draw.slot);
    assert!(slot.is_some());

    rig.service.block_changed([17, 64, 0]);
    rig.update(Some(&moved), &loaded);

    let jobs: Vec<(u32, i32)> = rig.frame.jobs.iter().map(|job| (job.slot, job.key.x)).collect();
    assert_eq!(jobs, [(slot.unwrap_or(u32::MAX), 1)]);
    assert_eq!(rig.full_draws(), shifted);
}

#[test]
fn a_shift_reads_no_layout_until_it_settles() {
    let mut rig = Rig::new();
    let marker = Rig::marker(1, Rotation::R0);
    let first = [(7, GhostFixture::structure([2, 3, 2], &["stone"; 12]))];
    let reloaded = [(7, GhostFixture::structure([2, 3, 2], &["stone"; 12]))];
    rig.update(Some(&marker), &[]);
    rig.run_to_rest(&marker, &first);
    let moved = Marker { origin: [1, 64, 0], ..marker.clone() };
    rig.update(Some(&moved), &first);

    rig.time += 0.1;
    rig.update(Some(&moved), &reloaded);

    assert!(rig.frame.jobs.is_empty());
    rig.time += GhostService::MOVE_SETTLE_SECONDS;
    rig.update(Some(&moved), &reloaded);
    assert!(!rig.frame.jobs.is_empty());
}

#[test]
fn a_move_settles_into_one_rebuild_after_the_quiet_time() {
    let mut rig = Rig::new();
    let marker = Rig::marker(1, Rotation::R0);
    let loaded = [(7, GhostFixture::structure([2, 3, 2], &["stone"; 12]))];
    rig.update(Some(&marker), &[]);
    rig.run_to_rest(&marker, &loaded);
    let generation = rig.frame.generation;
    let shifted = Rig::moved_draws(&rig.frame.draws, [1, 0, 0]);
    let moved = Marker { origin: [1, 64, 0], ..marker.clone() };
    rig.update(Some(&moved), &loaded);
    assert!(rig.frame.jobs.is_empty());

    rig.time += GhostService::MOVE_SETTLE_SECONDS;
    rig.update(Some(&moved), &loaded);

    assert!(!rig.frame.jobs.is_empty());
    assert_eq!(rig.frame.generation, generation);
    assert!(!rig.frame.clear_all);
    assert!(rig.frame.palette.is_none());
    assert_eq!(rig.frame.draws, shifted);
    let origins: Vec<(u32, [i32; 3])> = rig.frame.jobs.iter().map(|job| (job.slot, job.origin)).collect();

    rig.answer_jobs();
    rig.run_to_rest(&moved, &loaded);

    assert!(rig.frame.jobs.is_empty());
    assert_eq!(rig.frame.draws.len(), origins.len());
    for draw in &rig.frame.draws {
        assert!(origins.contains(&(draw.slot, draw.origin)), "{draw:?} is not at its job origin");
    }
    assert!(rig.frame.draws.iter().all(|draw| !shifted.contains(draw)));
}

#[test]
fn a_rotate_during_a_shift_rebuilds_at_once() {
    let mut rig = Rig::new();
    let marker = Rig::marker(1, Rotation::R0);
    let loaded = [(7, GhostFixture::structure([20, 1, 1], &["stone"; 20]))];
    rig.update(Some(&marker), &[]);
    rig.run_to_rest(&marker, &loaded);
    let shifted = Rig::moved_draws(&rig.full_draws(), [1, 0, 0]);
    let moved = Marker { origin: [1, 64, 0], ..marker.clone() };
    rig.update(Some(&moved), &loaded);
    assert!(rig.frame.jobs.is_empty());

    let turned = Marker { rotation: Rotation::R90, ..moved.clone() };
    rig.await_turn(&turned, &loaded);

    assert_eq!(rig.frame.rotation, Rotation::R90);
    assert!(!rig.frame.jobs.is_empty());
    assert!(rig.frame.palette.is_some());
    assert!(!rig.frame.clear_all);
    assert_eq!(rig.full_draws(), shifted);
}

#[test]
fn a_layer_view_during_a_shift_draws_the_same_layer_moved() {
    let mut rig = Rig::new();
    let marker = Rig::layer_marker(1);
    let loaded = [(7, GhostFixture::structure([2, 3, 2], &["stone"; 12]))];
    rig.update(Some(&marker), &[]);
    rig.run_to_rest(&marker, &loaded);
    let drawn = rig.full_draws();
    assert!(!drawn.is_empty());
    assert!(drawn.iter().all(|draw| draw.origin[1] == 65));

    let moved = Marker { origin: [0, 65, 0], ..marker.clone() };
    rig.update(Some(&moved), &loaded);

    assert!(rig.frame.jobs.is_empty());
    assert_eq!(rig.full_draws(), Rig::moved_draws(&drawn, [0, 1, 0]));
}

#[test]
fn the_draw_distance_uses_the_shifted_position() {
    let mut rig = Rig::with_config(GhostConfig { draw_distance: 1, ..GhostConfig::default() });
    rig.feet = [15, 64, 0];
    let marker = Marker { origin: [32, 64, 0], ..Rig::marker(1, Rotation::R0) };
    let loaded = [(7, GhostFixture::structure([1, 1, 1], &["stone"]))];
    rig.update(Some(&marker), &[]);
    let (keys, _, _) = rig.run_to_rest(&marker, &loaded);
    assert!(Rig::full_keys(&keys).is_empty());
    assert!(rig.full_draws().is_empty());

    let inside = Marker { origin: [31, 64, 0], ..marker.clone() };
    rig.update(Some(&inside), &loaded);
    assert_eq!(rig.frame.jobs.len(), 1);
    assert_eq!(rig.frame.jobs[0].key.x, 2);
    rig.answer_jobs();
    rig.update(Some(&inside), &loaded);
    let draws = rig.full_draws();
    assert_eq!(draws.len(), 1);
    assert_eq!(draws[0].origin, [31, 64, 0]);

    rig.time += GhostService::MOVE_SETTLE_SECONDS;
    rig.feet = [0, 64, 0];
    rig.run_to_rest(&inside, &loaded);
    assert_eq!(rig.full_draws().len(), 1);
    rig.update(Some(&marker), &loaded);
    assert!(rig.frame.jobs.is_empty());
    assert_eq!(rig.frame.frees.len(), 1);
    assert!(rig.full_draws().is_empty());
}

#[test]
fn rotation_change_makes_a_new_build() {
    let mut rig = Rig::new();
    let marker = Rig::marker(1, Rotation::R0);
    let loaded = [(7, GhostFixture::structure([20, 1, 1], &["stone"; 20]))];
    rig.update(Some(&marker), &[]);
    let (before, _, _) = rig.run_to_rest(&marker, &loaded);
    let drawn = rig.full_draws();
    let section = |key: &SectionKey| (key.x, key.y, key.z);
    let mut sections: Vec<_> = Rig::full_keys(&before).iter().map(section).collect();
    sections.sort();
    assert_eq!(sections, [(0, 4, 0), (1, 4, 0)]);

    let turned = Rig::marker(1, Rotation::R90);
    rig.update(Some(&turned), &loaded);
    assert_eq!(rig.full_draws(), drawn);
    assert_eq!(rig.frame.rotation, Rotation::R0);
    rig.await_turn(&turned, &loaded);
    assert_eq!(rig.full_draws(), drawn);
    assert!(rig.frame.palette.is_some());
    assert_eq!(rig.frame.rotation, Rotation::R90);
    assert!(!rig.frame.clear_all);
    let mut keys: HashSet<SectionKey> = rig.frame.jobs.iter().map(|job| job.key).collect();
    rig.answer_jobs();
    let (after, _, _) = rig.run_to_rest(&turned, &loaded);
    keys.extend(after);

    let mut sections: Vec<_> = Rig::full_keys(&keys).iter().map(section).collect();
    sections.sort();
    assert_eq!(sections, [(0, 4, 0), (0, 4, 1)]);
    assert_eq!(rig.full_draws().len(), 2);
}

#[test]
fn turned_copies_are_freed_when_the_build_turns_back_the_marker_changes_or_the_world_is_left() {
    let mut rig = Rig::new();
    let loaded = [
        (7, GhostFixture::structure([20, 1, 1], &["stone"; 20])),
        (8, GhostFixture::structure([3, 1, 1], &["dirt"; 3])),
    ];
    let straight = Rig::marker(1, Rotation::R0);
    let turned = Rig::marker(1, Rotation::R90);
    let other = Marker { target: MarkerTarget::Structure(StructureId(8)), ..Rig::marker(1, Rotation::R90) };
    rig.update(Some(&straight), &[]);
    rig.update(Some(&straight), &loaded);
    assert_eq!(rig.service.turned_bytes(), 0);

    rig.await_turn(&turned, &loaded);
    assert_eq!(rig.service.turned_bytes(), 40);
    rig.update(Some(&straight), &loaded);
    assert_eq!(rig.frame.rotation, Rotation::R0);
    assert_eq!(rig.service.turned_bytes(), 0);

    rig.await_turn(&turned, &loaded);
    assert_eq!(rig.service.turned_bytes(), 40);
    rig.update(Some(&other), &loaded);
    assert_eq!(rig.service.turned_bytes(), 0);
    rig.await_turn(&other, &loaded);
    assert_eq!(rig.service.turned_bytes(), 6);

    rig.await_turn(&turned, &loaded);
    assert_eq!(rig.service.turned_bytes(), 40);
    rig.update_in("W2", None, &[]);
    assert_eq!(rig.service.turned_bytes(), 0);
}

#[test]
fn shutdown_with_a_turn_in_flight_returns_at_once() {
    let mut rig = Rig::new();
    let side = 256_u32;
    let cells = (side * side * side) as usize;
    let big = Arc::new(Structure {
        size: StructureSize::new(side, side, side),
        world_origin: None,
        palette: vec![BlockState::new("minecraft:stone".to_string(), BlockStates::default())],
        primary: BlockLayer::new(vec![0; cells]),
        secondary: SparseLayer::default(),
    });
    let loaded = [(7, big)];
    let turned = Rig::marker(1, Rotation::R90);
    rig.update(Some(&turned), &[]);
    rig.update(Some(&turned), &loaded);
    assert!(rig.service.turning());
    std::thread::sleep(Duration::from_millis(20));

    let started = Instant::now();
    rig.service.shutdown();

    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(!rig.service.turning());
}

#[test]
fn whole_view_draws_the_miniature_and_no_marker() {
    let mut rig = Rig::new();
    let marker = Rig::marker(1, Rotation::R0);
    let loaded = [(7, GhostFixture::structure([20, 1, 1], &["stone"; 20]))];
    rig.update(Some(&marker), &[]);

    let (keys, _, _) = rig.run_to_rest(&marker, &loaded);

    let scale = (GhostSections::MINIATURE_SIZE / 20.0).min(GhostSections::MAX_FRACTION);
    let draws = rig.miniature_draws();
    assert_eq!(draws.len(), 1);
    assert_eq!(draws[0].origin, [0, 64, 0]);
    assert_eq!(draws[0].offset, [-20, 0, -1]);
    assert_eq!(draws[0].scale, scale);
    assert!(rig.marker_draws().is_empty());
    assert!(!keys.contains(&GhostSections::MARKER_KEY));
    assert!(!rig.slots.values().any(|key| key.part == SectionPart::Marker));
}

#[test]
fn miniature_covers_sections_past_the_draw_distance() {
    let mut rig = Rig::with_config(GhostConfig { draw_distance: 1, ..GhostConfig::default() });
    let marker = Rig::marker(1, Rotation::R0);
    let loaded = [(7, GhostFixture::structure([80, 1, 1], &["stone"; 80]))];
    rig.update(Some(&marker), &[]);

    let (keys, _, _) = rig.run_to_rest(&marker, &loaded);

    let miniature: Vec<SectionKey> = keys.iter().filter(|key| key.part.is_miniature()).copied().collect();
    assert_eq!(miniature, [MiniatureGroups::key(0)]);
    assert_eq!(Rig::member_offsets(&rig.group_jobs), [[0, 0, 0], [16, 0, 0], [32, 0, 0], [48, 0, 0], [64, 0, 0]]);
    assert_eq!(rig.service.miniature_slots(), 1);
    assert_eq!(rig.service.miniature_sections(), 5);
    assert_eq!(rig.miniature_draws().len(), 1);
    let mut full: Vec<i32> = Rig::full_keys(&keys).iter().map(|key| key.x).collect();
    full.sort();
    assert_eq!(full, [0, 1]);
}

#[test]
fn layer_view_draws_the_marker_and_no_miniature() {
    let mut rig = Rig::new();
    let marker = Rig::layer_marker(1);
    let loaded = [(7, GhostFixture::structure([2, 3, 2], &["stone"; 12]))];
    rig.update(Some(&marker), &[]);

    rig.run_to_rest(&marker, &loaded);

    let draws = rig.marker_draws();
    assert_eq!(draws.len(), 1);
    assert_eq!(draws[0].origin, [-1, 64, -1]);
    assert_eq!(draws[0].offset, [0; 3]);
    assert_eq!(draws[0].scale, 1.0);
    assert!(rig.miniature_draws().is_empty());
}

#[test]
fn layer_view_keeps_miniature_slots_undrawn() {
    let mut rig = Rig::new();
    let whole = Rig::marker(1, Rotation::R0);
    let layer = Rig::layer_marker(1);
    let loaded = [(7, GhostFixture::structure([2, 3, 2], &["stone"; 12]))];
    rig.update(Some(&whole), &[]);
    let (first, _, _) = rig.run_to_rest(&whole, &loaded);
    assert!(first.iter().any(|key| key.part.is_miniature()));
    let drawn = rig.miniature_draws();
    assert!(!drawn.is_empty());

    let (second, _, _) = rig.run_to_rest(&layer, &loaded);
    assert!(second.iter().all(|key| !key.part.is_miniature()));
    assert!(rig.miniature_draws().is_empty());
    assert_eq!(rig.marker_draws().len(), 1);

    let (third, _, _) = rig.run_to_rest(&whole, &loaded);
    assert!(third.iter().all(|key| !key.part.is_miniature()));
    assert_eq!(rig.miniature_draws(), drawn);
    assert!(rig.marker_draws().is_empty());
}

#[test]
fn miniature_switch_off_shows_the_marker_in_whole_view() {
    let mut rig = Rig::with_config(GhostConfig { miniature: false, ..GhostConfig::default() });
    let marker = Rig::marker(1, Rotation::R0);
    let loaded = [(7, GhostFixture::structure([2, 3, 2], &["stone"; 12]))];
    rig.update(Some(&marker), &[]);

    let (keys, _, _) = rig.run_to_rest(&marker, &loaded);

    assert_eq!(rig.marker_draws().len(), 1);
    assert!(keys.iter().all(|key| !key.part.is_miniature()));
    assert!(rig.miniature_draws().is_empty());
    assert!(!rig.service.miniature_over_limit());
}

#[test]
fn over_the_slot_limit_shows_the_marker() {
    let mut rig = Rig::with_config(GhostConfig { mesh_budget_mb: 1, ..GhostConfig::default() });
    let marker = Rig::marker(1, Rotation::R0);
    let loaded = [(7, GhostFixture::structure([20, 1, 1], &["stone"; 20]))];
    assert!(rig.service.slot_capacity() / GhostService::MINIATURE_SLOT_DIVISOR < 2);
    rig.update(Some(&marker), &[]);

    let (keys, _, _) = rig.run_to_rest(&marker, &loaded);

    assert_eq!(rig.marker_draws().len(), 1);
    assert!(keys.iter().all(|key| !key.part.is_miniature()));
    assert!(rig.miniature_draws().is_empty());
    assert!(rig.service.miniature_over_limit());
}

#[test]
fn a_block_change_does_not_rebuild_the_miniature() {
    let mut rig = Rig::new();
    let marker = Rig::marker(1, Rotation::R0);
    let loaded = [(7, GhostFixture::structure([20, 1, 1], &["stone"; 20]))];
    rig.update(Some(&marker), &[]);
    rig.run_to_rest(&marker, &loaded);

    rig.service.block_changed([5, 64, 0]);
    rig.update(Some(&marker), &loaded);

    assert!(!rig.full_jobs().is_empty());
    assert!(rig.frame.jobs.iter().all(|job| !job.key.part.is_miniature()));
}

#[test]
fn a_move_rebuilds_the_miniature() {
    let mut rig = Rig::new();
    let marker = Rig::marker(1, Rotation::R0);
    let loaded = [(7, GhostFixture::structure([20, 1, 1], &["stone"; 20]))];
    rig.update(Some(&marker), &[]);
    rig.run_to_rest(&marker, &loaded);

    let moved = Marker { origin: [1, 64, 0], ..marker.clone() };
    rig.update(Some(&moved), &loaded);
    rig.time += GhostService::MOVE_SETTLE_SECONDS;
    let mut keys: HashSet<SectionKey> = HashSet::new();
    for _ in 0..2 {
        rig.update(Some(&moved), &loaded);
        keys.extend(rig.frame.jobs.iter().map(|job| job.key));
        rig.answer_jobs();
    }
    let (rest, _, _) = rig.run_to_rest(&moved, &loaded);
    keys.extend(rest);

    assert_eq!(keys.iter().filter(|key| key.part.is_miniature()).count(), 1);
    assert_eq!(rig.miniature_draws().len(), 1);
    assert!(rig.miniature_draws().iter().all(|draw| draw.origin == [1, 64, 0]));
}

#[test]
fn hidden_marker_draws_neither() {
    let mut rig = Rig::new();
    let marker = Rig::marker(1, Rotation::R0);
    let loaded = [(7, GhostFixture::structure([20, 1, 1], &["stone"; 20]))];
    rig.update(Some(&marker), &[]);
    rig.run_to_rest(&marker, &loaded);
    assert!(!rig.miniature_draws().is_empty());

    rig.update_hidden(&marker, &loaded);

    assert!(rig.miniature_draws().is_empty());
    assert!(rig.marker_draws().is_empty());
    assert!(rig.frame.draws.is_empty());
}

#[test]
fn a_turn_of_the_build_hands_out_the_miniature_jobs_again() {
    let mut rig = Rig::new();
    let marker = Rig::marker(1, Rotation::R0);
    let turned = Rig::marker(1, Rotation::R90);
    let loaded = [(7, GhostFixture::structure([20, 1, 1], &["stone"; 20]))];
    rig.update(Some(&marker), &[]);
    rig.run_to_rest(&marker, &loaded);

    rig.await_turn(&turned, &loaded);
    let mut keys: HashSet<SectionKey> = rig.frame.jobs.iter().map(|job| job.key).collect();
    rig.answer_jobs();
    let (rest, _, _) = rig.run_to_rest(&turned, &loaded);
    keys.extend(rest);

    assert_eq!(keys.iter().filter(|key| key.part.is_miniature()).count(), 1);
    assert!(!rig.miniature_draws().is_empty());
    assert!(rig.marker_draws().is_empty());
}

#[test]
fn a_new_structure_for_the_marker_hands_out_new_miniature_jobs() {
    let mut rig = Rig::new();
    let first = Rig::marker(1, Rotation::R0);
    let second = Rig::marker(2, Rotation::R0);
    let loaded = [(7, GhostFixture::structure([20, 1, 1], &["stone"; 20]))];
    rig.update(Some(&first), &[]);
    rig.run_to_rest(&first, &loaded);
    let generation = rig.frame.generation;

    let (keys, _, cleared) = rig.run_to_rest(&second, &loaded);

    assert!(cleared);
    assert!(rig.frame.generation > generation);
    assert_eq!(keys.iter().filter(|key| key.part.is_miniature()).count(), 1);
    assert!(!rig.miniature_draws().is_empty());
}

#[test]
fn no_active_marker_draws_neither_marker_nor_miniature() {
    let mut rig = Rig::new();
    let marker = Rig::marker(1, Rotation::R0);
    let loaded = [(7, GhostFixture::structure([20, 1, 1], &["stone"; 20]))];
    rig.update(Some(&marker), &[]);
    rig.run_to_rest(&marker, &loaded);
    assert!(!rig.miniature_draws().is_empty());

    rig.update(None, &loaded);

    assert!(rig.frame.draws.is_empty());
    assert!(rig.miniature_draws().is_empty());
    assert!(rig.marker_draws().is_empty());
}

#[test]
fn leaving_the_world_draws_neither_marker_nor_miniature() {
    let mut rig = Rig::new();
    let marker = Rig::marker(1, Rotation::R0);
    let loaded = [(7, GhostFixture::structure([20, 1, 1], &["stone"; 20]))];
    rig.update(Some(&marker), &[]);
    rig.run_to_rest(&marker, &loaded);
    assert!(!rig.miniature_draws().is_empty());

    let status = Frames::main_menu(rig.time);
    let held = GhostFixture::loaded(&loaded);
    rig.service.update(Some(&marker), &rig.library, &held, &status, true, &mut rig.frame);

    assert!(!rig.frame.in_world);
    assert!(rig.frame.draws.is_empty());
    assert!(rig.miniature_draws().is_empty());
    assert!(rig.marker_draws().is_empty());
}

#[test]
fn a_moved_build_clears_the_over_limit_flag_while_it_is_rescanned() {
    let mut rig = Rig::with_config(GhostConfig { mesh_budget_mb: 1, ..GhostConfig::default() });
    let marker = Rig::marker(1, Rotation::R0);
    let loaded = [(7, GhostFixture::structure([272, 1, 1], &["stone"; 272]))];
    rig.update(Some(&marker), &[]);
    rig.run_to_rest(&marker, &loaded);
    assert!(rig.service.miniature_over_limit());

    let moved = Marker { origin: [1, 64, 0], ..marker.clone() };
    rig.update(Some(&moved), &loaded);
    rig.time += GhostService::MOVE_SETTLE_SECONDS;
    rig.update(Some(&moved), &loaded);

    assert!(rig.service.scanning());
    assert!(!rig.service.miniature_over_limit());
}

#[test]
fn a_settings_rescan_rebuilds_every_group() {
    let mut rig = Rig::new();
    let marker = Rig::marker(1, Rotation::R0);
    let names = vec!["stone"; 80 * 16 * 80];
    let loaded = [(7, GhostFixture::structure([80, 16, 80], &names))];
    rig.update(Some(&marker), &[]);
    rig.run_to_rest(&marker, &loaded);
    let before = std::mem::take(&mut rig.group_jobs);
    let groups = before.iter().map(|job| job.0).collect::<HashSet<_>>();
    assert!(groups.len() > 1);

    rig.service.reconfigure(GhostConfig { cells_per_frame: 2048, ..GhostConfig::default() });
    let (keys, _, _) = rig.run_to_rest(&marker, &loaded);

    for group in &groups {
        let old = before.iter().find(|job| job.0 == *group).map(|job| job.2).unwrap();
        let first = rig.group_jobs.iter().find(|job| job.0 == *group).unwrap();
        assert_eq!(first.3, 0);
        assert_ne!(first.2, old);
    }
    let occupied = Rig::full_keys(&keys);
    assert_eq!(Rig::member_offsets(&rig.group_jobs), Rig::key_offsets(&occupied));
    assert_eq!(rig.service.miniature_sections() as usize, occupied.len());
    assert_eq!(rig.service.miniature_slots() as usize, groups.len());

    let mut fresh = MiniatureGroups::new();
    fresh.cut(&occupied, &vec![4096; occupied.len()]);
    assert_eq!(fresh.group_count() as usize, groups.len());
    for group in 0..fresh.group_count() {
        let key = MiniatureGroups::key(group);
        let start = rig.group_jobs.iter().rposition(|job| job.0 == key && job.3 == 0).unwrap();
        let appends =
            rig.group_jobs[start..].iter().filter(|job| job.0 == key && job.1 == GhostJobMode::Append).count();
        assert_eq!(appends, fresh.members(group).len());
    }
}

#[test]
fn layer_view_keeps_building_the_groups_undrawn() {
    let mut rig = Rig::new();
    let whole = Rig::marker(1, Rotation::R0);
    let layer = Rig::layer_marker(1);
    let loaded = [(7, GhostFixture::structure([20, 3, 1], &["stone"; 60]))];
    rig.update(Some(&whole), &[]);
    for _ in 0..50 {
        rig.update(Some(&whole), &loaded);
        if rig.group_jobs.iter().any(|job| job.1 == GhostJobMode::Append) {
            break;
        }
        rig.answer_jobs();
    }
    rig.answer_jobs();
    assert_eq!(rig.service.miniature_slots(), 0);

    let (keys, _, _) = rig.run_to_rest(&layer, &loaded);

    assert!(keys.contains(&MiniatureGroups::key(0)));
    assert_eq!(rig.service.miniature_slots(), 1);
    assert!(rig.miniature_draws().is_empty());
    assert_eq!(rig.marker_draws().len(), 1);

    rig.update(Some(&whole), &loaded);

    assert_eq!(rig.miniature_draws().len(), 1);
}

#[test]
fn miniature_telemetry_counts_groups_and_sections() {
    let mut rig = Rig::new();
    let marker = Rig::marker(1, Rotation::R0);
    let names = vec!["stone"; 80 * 16 * 16];
    let loaded = [(7, GhostFixture::structure([80, 16, 16], &names))];
    rig.update(Some(&marker), &[]);

    let (keys, _, _) = rig.run_to_rest(&marker, &loaded);

    let finishes = rig.group_jobs.iter().filter(|job| job.1 == GhostJobMode::Finish).count();
    assert_eq!(rig.service.miniature_slots(), 2);
    assert_eq!(finishes, 2);
    assert_eq!(rig.service.miniature_sections() as usize, Rig::full_keys(&keys).len());
    assert_eq!(rig.service.miniature_bytes(), 2 * 64);
}

#[test]
fn no_target_while_a_move_settles() {
    let mut rig = Rig::new();
    let marker = Rig::marker(1, Rotation::R0);
    let loaded = [(7, GhostFixture::structure([2, 3, 2], &["stone"; 12]))];
    rig.update(Some(&marker), &[]);
    assert!(rig.service.aim_view().is_none(), "no build yet");
    rig.run_to_rest(&marker, &loaded);
    let (build, view) = rig.service.aim_view().unwrap();
    assert_eq!((build.world_min, view), ([0, 64, 0], stratum_core::structs::ghost::GhostView::Whole));

    let moved = Marker { origin: [1, 64, 0], ..marker.clone() };
    rig.update(Some(&moved), &loaded);
    assert!(rig.service.aim_view().is_none(), "the build still sits at the old place");
    rig.time += 0.1;
    rig.update(Some(&moved), &loaded);
    assert!(rig.service.aim_view().is_none());

    rig.time += GhostService::MOVE_SETTLE_SECONDS;
    rig.update(Some(&moved), &loaded);
    let (build, _) = rig.service.aim_view().unwrap();
    assert_eq!(build.world_min, [1, 64, 0]);
}

#[test]
fn a_hidden_build_has_no_aim_view() {
    let mut rig = Rig::new();
    let marker = Rig::marker(1, Rotation::R0);
    let loaded = [(7, GhostFixture::structure([2, 3, 2], &["stone"; 12]))];
    rig.update(Some(&marker), &[]);
    rig.run_to_rest(&marker, &loaded);
    assert!(rig.service.aim_view().is_some());

    rig.update_hidden(&marker, &loaded);
    assert!(rig.service.aim_view().is_none());
}
