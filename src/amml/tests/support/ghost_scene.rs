use std::time::{Duration, Instant};

use stratum_core::services::NbtWriter;
use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::ghost::{GhostReport, SlotBuilt};
use stratum_core::structs::marker::MarkerTarget;
use stratum_core::structs::nbt::{NbtCompound, NbtTag};

use super::{CoreSession, GhostSeen};

// Steps for tests that draw a ghost through the take and the report: one full stone structure imported and a marker placed at
// the player's feet (10, 64, -20), active.
pub struct GhostScene;

impl GhostScene {
    const DEADLINE: Duration = Duration::from_secs(60);
    pub const SIZE: [i32; 3] = [2, 2, 2];

    // A session in the world with the marker placed, ticked until a take holds jobs; that take is returned.
    pub fn start() -> (CoreSession, GhostSeen) {
        let session = CoreSession::new();
        Self::import(&session);
        Self::place(&session);
        let seen = Self::tick_until_jobs(&session, 1.0);
        (session, seen)
    }

    pub fn import(session: &CoreSession) {
        let path = session.data.path().join("tower.mcstructure");
        std::fs::write(&path, Self::stone(Self::SIZE)).unwrap();
        assert_eq!(session.handle().with_state(|state| state.files_picked(vec![path])), Some(()));
        let start = Instant::now();
        while session.shared().snapshot().unsorted.is_empty() {
            assert!(start.elapsed() < Self::DEADLINE, "the structure was not imported");
            assert_eq!(session.tick(&CoreSession::in_world()), Some(()));
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    pub fn place(session: &CoreSession) {
        let id = session.shared().snapshot().unsorted[0].id;
        let place = LibraryAction::PlaceMarker { name: "Home".to_string(), target: MarkerTarget::Structure(id) };
        session.shared().send(&mut vec![place]);
        assert_eq!(session.tick(&CoreSession::in_world()), Some(()));
    }

    // Ticks at `time` and takes after each tick until a take holds a job.
    pub fn tick_until_jobs(session: &CoreSession, time: f64) -> GhostSeen {
        let start = Instant::now();
        loop {
            assert_eq!(session.tick(&CoreSession::in_world_at(time)), Some(()));
            let seen = GhostSeen::take(session);
            if !seen.jobs.is_empty() {
                return seen;
            }
            assert!(start.elapsed() < Self::DEADLINE, "the ghost did not produce a job");
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    // Answers every job of `seen` as built with `bytes`.
    pub fn report_built(session: &CoreSession, seen: &GhostSeen, bytes: u64) -> Option<()> {
        let built: Vec<SlotBuilt> = seen
            .jobs
            .iter()
            .map(|(generation, job)| SlotBuilt {
                slot: job.slot,
                serial: job.serial,
                generation: *generation,
                bytes,
                ok: true,
            })
            .collect();
        session.handle().ghost().report(false, |report| Self::write(report, &built, false))
    }

    // Writes every field of a reused report: `built` and `world_reset`, and nothing drawn or resolved.
    pub fn write(report: &mut GhostReport, built: &[SlotBuilt], world_reset: bool) {
        report.built.clear();
        report.built.extend_from_slice(built);
        report.unknown_blocks = 0;
        report.default_state_blocks = 0;
        report.palette_resolved = false;
        report.build_us = 0;
        report.draw_us_sum = 0;
        report.draw_frames = 0;
        report.draw_us_max = 0;
        report.meshes_drawn = 0;
        report.cells_built = 0;
        report.skipped_cells = 0;
        report.culled = 0;
        report.world_reset = world_reset;
    }

    // Answers the jobs of `seen` as built with `bytes`, then ticks, takes and answers again until a take holds no job.
    // Returns that take, which holds the draws of everything built.
    pub fn finish(session: &CoreSession, seen: &GhostSeen, bytes: u64) -> GhostSeen {
        let mut seen = seen.clone();
        for _ in 0..32 {
            assert_eq!(Self::report_built(session, &seen, bytes), Some(()));
            assert_eq!(session.tick(&CoreSession::in_world()), Some(()));
            seen = GhostSeen::take(session);
            if seen.jobs.is_empty() {
                return seen;
            }
        }
        panic!("the ghost kept handing out jobs");
    }

    // `.mcstructure` bytes of a box of `size` cells, every one stone.
    pub fn stone(size: [i32; 3]) -> Vec<u8> {
        let volume = size.iter().product::<i32>() as usize;
        let stone = NbtCompound::new(vec![
            ("name".to_string(), NbtTag::String("minecraft:stone".to_string())),
            ("states".to_string(), NbtTag::Compound(NbtCompound::default())),
            ("version".to_string(), NbtTag::Int(18_168_865)),
        ]);
        let default = NbtCompound::new(vec![
            ("block_palette".to_string(), NbtTag::List(vec![NbtTag::Compound(stone)])),
            ("block_position_data".to_string(), NbtTag::Compound(NbtCompound::default())),
        ]);
        let layers = NbtTag::List(vec![NbtTag::IntList(vec![0; volume]), NbtTag::IntList(vec![-1; volume])]);
        let palette = NbtCompound::new(vec![("default".to_string(), NbtTag::Compound(default))]);
        let structure = NbtCompound::new(vec![
            ("block_indices".to_string(), layers),
            ("entities".to_string(), NbtTag::List(Vec::new())),
            ("palette".to_string(), NbtTag::Compound(palette)),
        ]);
        NbtWriter::compound(
            "",
            &[
                ("format_version", NbtTag::Int(1)),
                ("size", NbtTag::IntList(size.to_vec())),
                ("structure", NbtTag::Compound(structure)),
                ("structure_world_origin", NbtTag::IntList(vec![0, 0, 0])),
            ],
        )
        .unwrap()
    }
}
