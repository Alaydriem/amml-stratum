use std::path::Path;
use std::time::{Duration, Instant};

use stratum_core::config::StratumConfig;
use stratum_core::services::StratumController;
use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::ghost::{GhostReport, SectionPart, SlotBuilt};
use stratum_core::structs::library::StructureId;
use stratum_core::structs::marker::MarkerTarget;

use super::{ControllerHarness, Frames, StructureFixture};

// Steps for controller tests that draw a ghost: a world with one imported structure and a placed marker.
pub struct GhostScene;

impl GhostScene {
    const DEADLINE: Duration = Duration::from_secs(60);
    pub const WORLD: &'static str = "W1";

    // A controller with `config`, one full stone structure of `size` imported, in world `W1` at time 1.0
    // with the marker "Home" placed at the player's feet (0, 64, 0) and active.
    pub fn start(data: &Path, config: StratumConfig, size: [i32; 3]) -> StratumController {
        let mut controller = StratumController::new(data.to_path_buf(), config);
        let bytes = StructureFixture::full(size).bytes();
        let path = ControllerHarness::picked_file(data, "tower.mcstructure", &bytes);
        controller.files_picked(vec![path]);
        ControllerHarness::settle(&mut controller, &Frames::main_menu(0.0));
        controller.tick(&Frames::in_world(Self::WORLD, 1.0));
        Self::place(&mut controller);
        controller
    }

    pub fn structure_id(controller: &StratumController) -> StructureId {
        controller.snapshot().unsorted[0].id
    }

    pub fn place(controller: &mut StratumController) {
        let target = MarkerTarget::Structure(Self::structure_id(controller));
        controller.apply(LibraryAction::PlaceMarker { name: "Home".to_string(), target });
    }

    // Ticks at `time` until the ghost frame holds a job.
    pub fn tick_until_jobs(controller: &mut StratumController, time: f64) {
        let start = Instant::now();
        loop {
            controller.tick(&Frames::in_world(Self::WORLD, time));
            if !controller.ghost_frame().jobs.is_empty() {
                return;
            }
            assert!(start.elapsed() < Self::DEADLINE, "the ghost did not produce a job");
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    // Ticks at `time` until the ghost frame carries a palette.
    pub fn tick_until_palette(controller: &mut StratumController, time: f64) {
        let start = Instant::now();
        loop {
            controller.tick(&Frames::in_world(Self::WORLD, time));
            if controller.ghost_frame().palette.is_some() {
                return;
            }
            assert!(start.elapsed() < Self::DEADLINE, "the ghost did not produce a palette");
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    // Answers every job of the current ghost frame as built.
    pub fn report_all(controller: &mut StratumController, bytes: u64) {
        let frame = controller.ghost_frame();
        let mut report = GhostReport::new();
        for job in &frame.jobs {
            report.built.push(SlotBuilt {
                slot: job.slot,
                serial: job.serial,
                generation: frame.generation,
                bytes,
                ok: true,
            });
        }
        controller.apply_ghost_report(&report);
    }

    // Reports the frame's jobs as built, then ticks once more at `time`.
    pub fn settle(controller: &mut StratumController, time: f64) {
        Self::report_all(controller, 1000);
        controller.tick(&Frames::in_world(Self::WORLD, time));
    }

    // Settles until a tick hands out no job, so the mini ghost groups are built too.
    pub fn rest(controller: &mut StratumController, time: f64) {
        for _ in 0..64 {
            Self::settle(controller, time);
            if controller.ghost_frame().jobs.is_empty() {
                return;
            }
        }
        panic!("the ghost did not come to rest");
    }

    // True for the parts of the full ghost: not the mini ghost and not the marker.
    pub fn is_full(part: SectionPart) -> bool {
        !part.is_miniature() && part != SectionPart::Marker
    }

    // The section X coordinates of the frame's full ghost jobs, sorted.
    pub fn job_xs(controller: &StratumController) -> Vec<i32> {
        let mut xs: Vec<i32> = controller
            .ghost_frame()
            .jobs
            .iter()
            .filter(|job| Self::is_full(job.key.part))
            .map(|job| job.key.x)
            .collect();
        xs.sort_unstable();
        xs
    }
}
