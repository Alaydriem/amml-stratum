use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use stratum_core::config::StratumConfig;
use stratum_core::services::StratumController;
use stratum_core::structs::frame::FrameStatus;

use super::Frames;

// Helpers for controller tests: a data folder, fixture files and waiting for imports.
pub struct ControllerHarness;

impl ControllerHarness {
    const DEADLINE: Duration = Duration::from_secs(60);

    pub fn config(max_structures: usize, queue: usize) -> StratumConfig {
        let mut config = StratumConfig::default();
        config.library.max_structures = max_structures;
        config.import.queue = queue;
        config
    }

    pub fn start(data: &Path) -> StratumController {
        StratumController::new(data.to_path_buf(), StratumConfig::default())
    }

    pub fn inbox_file(data: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let inbox = data.join("import");
        std::fs::create_dir_all(&inbox).unwrap();
        let path = inbox.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    pub fn picked_file(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    // Ticks with the same frame until no import is waiting for a result, then once more.
    pub fn settle(controller: &mut StratumController, frame: &FrameStatus) {
        let start = Instant::now();
        while controller.pending_imports() > 0 {
            assert!(start.elapsed() < Self::DEADLINE, "imports did not finish");
            controller.tick(frame);
            std::thread::yield_now();
        }
        controller.tick(frame);
    }

    // Ticks forward in time, two seconds a tick, until the unsorted list holds `count` structures.
    pub fn tick_until_unsorted(controller: &mut StratumController, world: &str, start_time: f64, count: usize) {
        let start = Instant::now();
        let mut time = start_time;
        while controller.snapshot().unsorted.len() < count {
            assert!(start.elapsed() < Self::DEADLINE, "inbox files did not arrive");
            controller.tick(&Frames::in_world(world, time));
            time += 2.0;
            std::thread::yield_now();
        }
    }

    // Ticks forward in time, one second a tick, until `path` no longer exists.
    pub fn tick_until_gone(controller: &mut StratumController, path: &Path, start_time: f64) {
        let start = Instant::now();
        let mut time = start_time;
        while path.exists() {
            assert!(start.elapsed() < Self::DEADLINE, "the file was not removed");
            controller.tick(&Frames::main_menu(time));
            time += 1.0;
            std::thread::yield_now();
        }
    }

    // Ticks until the material list being built is done, or failed.
    pub fn tick_until_materials_ready(controller: &mut StratumController, frame: &FrameStatus) {
        let start = Instant::now();
        while controller.snapshot().materials_loading.is_some() {
            assert!(start.elapsed() < Self::DEADLINE, "material list did not arrive");
            controller.tick(frame);
            std::thread::yield_now();
        }
    }

    // Waits until the queued saves reached the disk.
    pub fn flush(controller: &StratumController) {
        assert!(controller.flush_saves(Self::DEADLINE), "saves did not finish");
    }

    // Waits until the store writer counts at least `count` failed writes.
    pub fn wait_for_failures(controller: &StratumController, count: u64) {
        let start = Instant::now();
        while controller.store_failures() < count {
            assert!(start.elapsed() < Self::DEADLINE, "the write did not fail");
            std::thread::yield_now();
        }
    }

    pub fn unsorted_names(controller: &StratumController) -> Vec<String> {
        controller.snapshot().unsorted.iter().map(|row| row.name.clone()).collect()
    }

    pub fn file_names(dir: &Path) -> Vec<String> {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return Vec::new();
        };
        let mut names: Vec<String> = entries.map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        names.sort();
        names
    }
}
