#![allow(clippy::expect_used)]

use std::path::Path;
use std::time::{Duration, Instant};

use criterion::{Criterion, criterion_group, criterion_main};
use stratum_core::config::StratumConfig;
use stratum_core::services::StratumController;
use stratum_core::structs::action::LibraryAction;
use stratum_core::structs::frame::FrameStatus;
use stratum_core::structs::marker::MarkerTarget;

// A controller in a world with one imported sample and `MARKERS` markers on it.
struct MoveScene;

impl MoveScene {
    const MARKERS: usize = 64;
    const WORLD: &'static str = "bench";

    fn frame(feet: [i32; 3], time_seconds: f64) -> FrameStatus {
        FrameStatus {
            in_world: true,
            has_player: true,
            menu_open: false,
            world: Self::WORLD.to_string(),
            dimension: 0,
            feet,
            time_seconds,
            render_distance: 12,
        }
    }

    fn start(data: &Path) -> StratumController {
        let mut controller = StratumController::new(data.to_path_buf(), StratumConfig::default());
        let sample = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/tint.mcstructure");
        let picked = data.join("tint.mcstructure");
        std::fs::copy(sample, &picked).expect("sample copies");
        controller.files_picked(vec![picked]);
        let deadline = Instant::now() + Duration::from_secs(30);
        while controller.pending_imports() > 0 {
            assert!(Instant::now() < deadline, "import did not finish");
            controller.tick(&Self::frame([0, 64, 0], 0.0));
            std::thread::yield_now();
        }
        controller.tick(&Self::frame([0, 64, 0], 1.0));
        let target = MarkerTarget::Structure(controller.snapshot().unsorted[0].id);
        for index in 0..Self::MARKERS {
            controller.apply(LibraryAction::PlaceMarker { name: format!("marker {index}"), target });
            controller.tick(&Self::frame([index as i32, 64, 0], 2.0 + index as f64));
        }
        assert_eq!(controller.snapshot().markers.len(), Self::MARKERS);
        controller
    }
}

// One game tick in which the player moved, with 64 markers in the world. `move_tick_64_markers` is the tick as it
// runs; `move_tick_rows_rebuilt` also asks for a snapshot refresh, which builds and compares the marker rows.
fn bench_move_tick(c: &mut Criterion) {
    let mut group = c.benchmark_group("stratum_controller");
    let data = tempfile::tempdir().expect("temp dir");
    let mut controller = MoveScene::start(data.path());
    let mut step = 0i32;
    let mut time = 1_000.0;
    group.bench_function("move_tick_64_markers", |b| {
        b.iter(|| {
            step = step.wrapping_add(1);
            time += 0.05;
            controller.tick(&MoveScene::frame([step & 1023, 64, -(step & 511)], time));
        })
    });
    group.bench_function("move_tick_rows_rebuilt_64_markers", |b| {
        b.iter(|| {
            step = step.wrapping_add(1);
            time += 0.05;
            controller.files_picked(Vec::new());
            controller.tick(&MoveScene::frame([step & 1023, 64, -(step & 511)], time));
        })
    });
    group.finish();
}

criterion_group!(benches, bench_move_tick);
criterion_main!(benches);
