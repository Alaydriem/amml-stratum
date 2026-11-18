use std::sync::Arc;
use stratum_core::config::{BuildConfig, LibraryConfig};

use stratum_core::services::{BuildOrderWorker, BuildService, LibraryService, MarkerService};
use stratum_core::structs::action::BuildAction;
use stratum_core::structs::build::{BuildConditions, BuildPhase, BuildSettings, BuildSettingsBook};
use stratum_core::structs::frame::FrameStatus;
use stratum_core::structs::ghost::GhostBuild;
use stratum_core::structs::library::{Library, StructureId};
use stratum_core::structs::marker::{MarkerSet, MarkerTarget, Rotation, WorldKey};
use stratum_core::structs::structure::Structure;

use super::{BuildFixture, Frames, PieceFixture, Pieces};

// A build service, one library structure of 8 stones in a row, an active marker that shows it and the structure
// of that marker. The order worker works on the calling thread. The conditions allow a build.
pub struct BuildScene {
    pub service: BuildService,
    pub markers: MarkerService,
    pub library: LibraryService,
    pub structure: Arc<Structure>,
    pub time: f64,
}

impl BuildScene {
    pub const ALLOWED: BuildConditions = BuildConditions { local: true, cheats: true, operator: true, creative: true };

    pub fn new() -> Self {
        Self::with_build(BuildFixture::build(Pieces::layer_at(&["########"], [0, 64, 0]), Rotation::R0), [8, 1, 1])
    }

    pub fn with_build(build: GhostBuild, size: [u32; 3]) -> Self {
        let mut library = Library::new();
        library.structures.push(PieceFixture::at(1, [0, 0, 0], size));
        let mut markers = MarkerService::new(
            MarkerSet::new(WorldKey { world: BuildFixture::WORLD.to_string(), dimension: 0 }),
            stratum_core::config::MarkerConfig { max_per_world: 8 },
        );
        let id = markers.place("home", [0, 64, 0], MarkerTarget::Structure(StructureId(1))).unwrap();
        markers.activate(id).unwrap();
        let mut service =
            BuildService::with_worker(BuildConfig::default(), BuildSettingsBook::new(8), BuildOrderWorker::inline());
        service.set_conditions(Self::ALLOWED);
        Self {
            service,
            markers,
            library: LibraryService::new(library, LibraryConfig::default()),
            structure: Arc::clone(&build.pieces[0].structure),
            time: 1.0,
        }
    }

    pub fn status(&self) -> FrameStatus {
        Frames::in_world(BuildFixture::WORLD, self.time)
    }

    // Moves the clock by `seconds`, ticks the service with the ghost build, and returns the commands it queued.
    pub fn tick(&mut self, seconds: f64) -> Vec<String> {
        self.time += seconds;
        let status = self.status();
        self.service.tick(&status, Some(&self.structure), Some(&self.markers), &self.library);
        self.drain()
    }

    pub fn tick_with(&mut self, status: &FrameStatus) -> Vec<String> {
        self.service.tick(status, Some(&self.structure), Some(&self.markers), &self.library);
        self.drain()
    }

    pub fn drain(&mut self) -> Vec<String> {
        let mut out = Vec::new();
        self.service.take_commands(&mut out);
        out
    }

    pub fn apply(&mut self, action: BuildAction) {
        self.service.apply(action, self.time, Some(&self.markers), &self.library);
    }

    // Opens the structure, lets the build attach, and answers the probe with every block known.
    pub fn open_ready(&mut self, settings: BuildSettings) {
        self.apply(BuildAction::SetSettings(StructureId(1), settings));
        self.apply(BuildAction::Open(StructureId(1)));
        let request = self.probe_request();
        self.service.apply_probe(&BuildFixture::exact_entries(request.entries.len()));
        self.settle();
        assert_eq!(self.phase(), Some(BuildPhase::Ready));
    }

    // Ticks until the worker has placed the structure and the palette request is there.
    pub fn probe_request(&mut self) -> stratum_core::structs::build::BuildProbeRequest {
        for _ in 0..8 {
            self.tick(0.0);
            if let Some(request) = self.service.take_probe() {
                return request;
            }
        }
        panic!("no palette request");
    }

    // Ticks until the worker has counted the blocks.
    pub fn settle(&mut self) {
        for _ in 0..8 {
            self.tick(0.0);
            if self.phase() == Some(BuildPhase::Ready) {
                return;
            }
        }
    }

    // Starts a build with the default settings and ticks until the clear is sent.
    pub fn running(&mut self) {
        self.open_ready(BuildSettings::default());
        self.apply(BuildAction::Start);
        for _ in 0..8 {
            self.tick(0.25);
            if self.phase() == Some(BuildPhase::Building) {
                return;
            }
        }
        panic!("the build did not leave the clear");
    }

    pub fn phase(&self) -> Option<BuildPhase> {
        self.service.row().map(|row| row.phase)
    }
}
