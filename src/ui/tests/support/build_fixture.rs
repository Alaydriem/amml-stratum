use stratum_core::structs::build::{BuildPhase, BuildSettings};
use stratum_core::structs::library::StructureId;
use stratum_core::structs::snapshot::BuildRow;

// Build rows the build view tests share: "Main hall" (id 1) as on the build Paper sheets.
pub struct BuildFixture;

impl BuildFixture {
    // The panel row of a structure the game has looked up: 4,212 blocks, the default settings, nothing unplaceable.
    pub fn ready() -> BuildRow {
        BuildRow {
            structure: StructureId(1),
            structure_name: "Main hall".to_owned(),
            phase: BuildPhase::Ready,
            settings: BuildSettings::default(),
            layer: 0,
            layers: 18,
            placed: 0,
            total: 4212,
            eta_seconds: 1053,
            unplaceable: Vec::new(),
            notice: None,
            confirm_stop: false,
        }
    }

    // The card row of layer 7 of 18 with 1,640 of 4,212 blocks placed, 8 min 12 s left at 4 blocks a second.
    pub fn running(phase: BuildPhase) -> BuildRow {
        BuildRow { phase, layer: 7, placed: 1640, eta_seconds: 492, ..Self::ready() }
    }
}
