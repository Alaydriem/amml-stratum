use std::sync::Arc;
use std::time::Instant;

use stratum_core::services::{GhostSections, SectionScan};
use stratum_core::structs::ghost::{GhostBuild, GhostPalette, GhostPiece};
use stratum_core::structs::structure::{BlockLayer, SparseLayer, Structure, StructureSize};

use crate::support::{BlockFactory, GhostFixture};

// Builds of several pieces for scan tests.
struct Pieces;

impl Pieces {
    fn build(size: [u32; 3], origins: &[[i32; 3]], structure: Arc<Structure>) -> GhostBuild {
        let mut min = origins[0];
        let mut max = origins[0];
        for origin in origins {
            for axis in 0..3 {
                min[axis] = min[axis].min(origin[axis]);
                max[axis] = max[axis].max(origin[axis] + size[axis] as i32 - 1);
            }
        }
        let mut palette = GhostPalette { entries: vec![BlockFactory::plain("stone")] };
        let marker_block = palette.marker_id().unwrap();
        GhostBuild {
            pieces: origins
                .iter()
                .map(|origin| GhostPiece { structure: structure.clone(), world_min: *origin, remap: vec![0] })
                .collect(),
            palette,
            marker_block,
            world_min: min,
            world_max: max,
            revision: 1,
        }
    }

    fn full(size: [u32; 3]) -> Arc<Structure> {
        let cells = (size[0] * size[1] * size[2]) as usize;
        Arc::new(Structure {
            size: StructureSize::new(size[0], size[1], size[2]),
            world_origin: None,
            palette: vec![BlockFactory::plain("stone")],
            primary: BlockLayer::new(vec![0; cells]),
            secondary: SparseLayer::default(),
        })
    }
}

#[test]
fn a_scan_in_steps_finds_the_keys_of_one_full_scan() {
    let build = GhostFixture::blocks(&[([0, 0, 0], [79, 15, 79])]);
    let mut whole = Vec::new();
    let mut whole_cells = Vec::new();
    GhostSections::occupied(&build, 600, &mut whole, &mut whole_cells);

    let mut scan = SectionScan::new(&build);
    let mut found = Vec::new();
    let mut found_cells = Vec::new();
    let mut steps = 1;
    while !scan.step(&build, 600, 16, &mut found, &mut found_cells) {
        steps += 1;
    }

    assert!(steps > 1);
    assert!(scan.done());
    assert_eq!(found_cells.len(), found.len());
    assert_eq!(found_cells.iter().sum::<u32>(), 80 * 16 * 80);
    assert_eq!(found, whole);
    assert_eq!(found_cells, whole_cells);
}

#[test]
fn a_step_never_reads_more_sections_than_its_budget() {
    let build = GhostFixture::blocks(&[([0, 0, 0], [79, 15, 79])]);
    let mut scan = SectionScan::new(&build);
    let mut found = Vec::new();

    scan.step(&build, 600, 3, &mut found, &mut Vec::new());

    // Each section of 4096 cells is cut into two halves above 2 x 600 cells.
    assert_eq!(found.len(), 6);
    assert!(!scan.done());
}

#[test]
fn sections_that_no_piece_reaches_cost_no_budget() {
    let build = Pieces::build([16, 16, 16], &[[0, 0, 0], [160, 0, 0]], Pieces::full([16, 16, 16]));
    let mut scan = SectionScan::new(&build);
    let mut found = Vec::new();

    let first = scan.step(&build, 6000, 1, &mut found, &mut Vec::new());
    assert!(!first);
    assert_eq!(found.iter().map(|key| key.x).collect::<Vec<_>>(), [0]);

    let second = scan.step(&build, 6000, 1, &mut found, &mut Vec::new());
    assert!(second);
    assert_eq!(found.iter().map(|key| key.x).collect::<Vec<_>>(), [0, 10]);
}

// The target build: 8 pieces of 64 x 192 x 64. Run with the ignored tests, in release.
#[test]
#[ignore = "timing run, release only"]
fn scan_speed_of_the_target_build() {
    let origins: Vec<[i32; 3]> = (0..8).map(|n| [(n % 4) * 64, 0, (n / 4) * 64]).collect();
    let build = Pieces::build([64, 192, 64], &origins, Pieces::full([64, 192, 64]));
    let cells = 8_u64 * 64 * 192 * 64;
    let mut found = Vec::new();

    let start = Instant::now();
    let mut scan = SectionScan::new(&build);
    while !scan.step(&build, 600, usize::MAX, &mut found, &mut Vec::new()) {}
    let elapsed = start.elapsed();

    println!(
        "scan of {cells} cells: {} ms, {:.2} ns per cell, {} keys",
        elapsed.as_millis(),
        elapsed.as_nanos() as f64 / cells as f64,
        found.len()
    );
    assert!(!found.is_empty());
}
