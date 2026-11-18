use stratum_core::structs::build::{BuildAnchor, BuildProbeResult, PaletteOutcome};
use stratum_core::structs::ghost::{GhostBuild, GhostPalette, GhostPiece};
use stratum_core::structs::library::StructureId;
use stratum_core::structs::marker::{MarkerId, Rotation};

// Makes ghost builds, anchors and probe answers for build tests.
pub struct BuildFixture;

impl BuildFixture {
    pub const WORLD: &'static str = "W1";

    // A build of one piece: the palette holds the piece palette without air and structure void, and the box is the
    // piece box.
    pub fn build(piece: GhostPiece, rotation: Rotation) -> GhostBuild {
        let mut palette = GhostPalette::new();
        let mut remap = Vec::new();
        for block in &piece.structure.palette {
            if block.is_empty_block() {
                remap.push(GhostPalette::NONE);
            } else {
                remap.push(palette.entries.len() as u16);
                palette.entries.push(block.clone());
            }
        }
        let marker_block = palette.marker_id().unwrap();
        let size = piece.structure.size;
        let min = piece.world_min;
        let max = [min[0] + size.x as i32 - 1, min[1] + size.y as i32 - 1, min[2] + size.z as i32 - 1];
        let piece = GhostPiece { remap, ..piece };
        GhostBuild::new(vec![piece], palette, marker_block, min, max, 1, rotation)
    }

    pub fn anchor(structure: u64) -> BuildAnchor {
        BuildAnchor {
            world: Self::WORLD.to_string(),
            dimension: 0,
            marker: MarkerId(1),
            origin: [0, 64, 0],
            rotation: Rotation::R0,
            structure: StructureId(structure),
        }
    }

    // The game knows every block of the palette.
    pub fn exact(build: &GhostBuild) -> BuildProbeResult {
        BuildProbeResult {
            outcomes: vec![PaletteOutcome::Exact; build.palette.entries.len()],
            turn_mismatches: Vec::new(),
        }
    }

    // The game knows every one of `entries` palette entries.
    pub fn exact_entries(entries: usize) -> BuildProbeResult {
        BuildProbeResult { outcomes: vec![PaletteOutcome::Exact; entries], turn_mismatches: Vec::new() }
    }

    // The game does not know the blocks named in `unknown`, and knows the rest.
    pub fn unknown(build: &GhostBuild, unknown: &[&str]) -> BuildProbeResult {
        let outcomes = build
            .palette
            .entries
            .iter()
            .map(|block| {
                if unknown.iter().any(|name| block.name == format!("minecraft:{name}")) {
                    PaletteOutcome::Unknown
                } else {
                    PaletteOutcome::Exact
                }
            })
            .collect();
        BuildProbeResult { outcomes, turn_mismatches: Vec::new() }
    }

    // The coordinates of a `/setblock` or `/fill` command, the first three numbers after the command name.
    pub fn position(command: &str) -> [i32; 3] {
        let numbers: Vec<i32> = command.split_whitespace().skip(1).take(3).map(|part| part.parse().unwrap()).collect();
        [numbers[0], numbers[1], numbers[2]]
    }
}
