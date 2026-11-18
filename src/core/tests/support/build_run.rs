use stratum_core::config::BuildConfig;
use stratum_core::services::{BuildOrderWorker, BuildSession};
use stratum_core::structs::build::{BuildOutbox, BuildPhase, BuildSettings, BuildStopReason};
use stratum_core::structs::ghost::GhostBuild;
use stratum_core::structs::library::StructureId;
use stratum_core::structs::snapshot::BuildRow;

use super::BuildFixture;

// Drives one build session with explicit time: the session, its outbox and a clock.
pub struct BuildRun {
    pub session: BuildSession,
    pub outbox: BuildOutbox,
    pub time: f64,
    // False: layers are worked out only when a test asks, with `pump`.
    pub auto: bool,
}

impl BuildRun {
    // A session opened on `build`, with the game knowing every block, in the `Ready` phase.
    pub fn ready(build: &GhostBuild, settings: BuildSettings) -> Self {
        let probe = BuildFixture::exact(build);
        Self::probed(build, settings, &probe)
    }

    pub fn probed(
        build: &GhostBuild,
        settings: BuildSettings,
        probe: &stratum_core::structs::build::BuildProbeResult,
    ) -> Self {
        let config = BuildConfig::default();
        let mut session = BuildSession::new(config.kinds_capacity(), StructureId(1), "tower".to_string(), settings);
        session.attach(build, BuildFixture::anchor(1));
        assert!(session.take_probe().is_some());
        session.apply_probe(probe);
        let mut run = Self { session, outbox: BuildOutbox::new(config.outbox_capacity()), time: 0.0, auto: true };
        // The worker counts the blocks; the session is ready once the count is back.
        run.pump();
        run
    }

    pub fn start(&mut self) {
        self.session.start(self.time, &mut self.outbox);
        self.pump_if_auto();
    }

    // Works out the layer the session asks for and hands it back.
    pub fn pump(&mut self) {
        while let Some(job) = self.session.take_job() {
            self.session.apply_answer(BuildOrderWorker::perform(job));
        }
    }

    fn pump_if_auto(&mut self) {
        if self.auto {
            self.pump();
        }
    }

    // Moves the clock by `seconds`, ticks, and returns the commands the tick queued.
    pub fn tick(&mut self, seconds: f64) -> Vec<String> {
        self.time += seconds;
        self.pump_if_auto();
        self.session.tick(self.time, &mut self.outbox);
        self.pump_if_auto();
        self.drain()
    }

    pub fn drain(&mut self) -> Vec<String> {
        let mut out = Vec::new();
        self.outbox.drain(&mut out);
        out
    }

    // Starts the build and ticks until the clear is sent; returns every command queued on the way.
    pub fn start_and_clear(&mut self) -> Vec<String> {
        self.start();
        let mut commands = self.drain();
        for _ in 0..64 {
            if self.phase() != BuildPhase::Clearing {
                break;
            }
            commands.extend(self.tick(0.25));
        }
        assert_eq!(self.phase(), BuildPhase::Building);
        commands
    }

    pub fn phase(&self) -> BuildPhase {
        self.session.phase()
    }

    pub fn row(&self) -> BuildRow {
        self.session.row()
    }

    pub fn stop(&mut self, reason: BuildStopReason) {
        self.session.stop(reason, &mut self.outbox);
    }

    // True for a `/setblock` command.
    pub fn is_setblock(command: &str) -> bool {
        command.starts_with("/setblock ")
    }
}
