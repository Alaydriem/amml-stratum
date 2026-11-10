use stratum_core::config::GhostConfig;
use stratum_core::services::{GhostScheduler, GhostSections, LayoutSolver, StructureParser};
use stratum_core::structs::ghost::{GhostFrame, GhostReport, GhostView, MiniatureGroups, SlotBuilt};
use stratum_core::structs::layout::PiecePlacement;
use stratum_core::structs::library::StructureEntry;
use stratum_core::structs::structure::{Structure, StructureSize};
use stratum_tools::SampleBuilder;

use crate::entry_fixture::EntryFixture;
use crate::target_build::TargetBuild;

#[test]
fn large_sample_parses_with_the_spec_size_and_fill() {
    let large = StructureParser::parse(&SampleBuilder::large().unwrap()).unwrap();
    assert_eq!(large.size, StructureSize::new(128, 64, 128));
    let fill = large.block_count() as f64 / large.size.volume() as f64;
    assert!((0.20..=0.30).contains(&fill), "fill rate {fill}");
    let names: Vec<&str> = large.palette.iter().map(|block| block.name.as_str()).collect();
    for expected in [
        "minecraft:stone",
        "minecraft:oak_stairs",
        "minecraft:oak_slab",
        "minecraft:oak_fence",
        "minecraft:glass_pane",
        "minecraft:short_grass",
    ] {
        assert!(names.contains(&expected), "missing {expected}");
    }
}

#[test]
fn standard_sample_is_a_tower_with_the_large_samples_fill_and_blocks() {
    let standard = StructureParser::parse(&SampleBuilder::standard().unwrap()).unwrap();
    assert_eq!(standard.size, StructureSize::new(64, 192, 64));
    let fill = standard.block_count() as f64 / standard.size.volume() as f64;
    assert!((0.20..=0.30).contains(&fill), "fill rate {fill}");
    let large = StructureParser::parse(&SampleBuilder::large().unwrap()).unwrap();
    assert_eq!(standard.palette, large.palette);
    // the same rule: a stone shell, plank floors every 8 levels
    assert_eq!(standard.block_at(0, 100, 30).map(|block| block.name.as_str()), Some("minecraft:stone"));
    assert_eq!(standard.block_at(30, 184, 30).map(|block| block.name.as_str()), Some("minecraft:oak_planks"));
}

#[test]
fn pieces_align_into_the_large_sample_with_no_seams() {
    let large = StructureParser::parse(&SampleBuilder::large().unwrap()).unwrap();
    let pieces: Vec<Structure> =
        SampleBuilder::pieces().unwrap().iter().map(|bytes| StructureParser::parse(bytes).unwrap()).collect();
    for piece in &pieces {
        assert_eq!(piece.size, StructureSize::new(64, 64, 64));
    }
    let entries: Vec<StructureEntry> =
        pieces.iter().enumerate().map(|(index, piece)| EntryFixture::of(index as u64 + 1, piece)).collect();
    let refs: Vec<&StructureEntry> = entries.iter().collect();
    let layout = LayoutSolver::from_origins(&refs).unwrap();
    assert_eq!(layout.size, StructureSize::new(128, 64, 128));

    let mut offsets: Vec<[i32; 3]> = layout.pieces.iter().map(|placement| placement.offset).collect();
    offsets.sort();
    assert_eq!(offsets, [[0, 0, 0], [0, 0, 64], [64, 0, 0], [64, 0, 64]]);

    for PiecePlacement { structure, offset } in &layout.pieces {
        let piece = &pieces[structure.0 as usize - 1];
        for x in 0..64 {
            for y in 0..64 {
                for z in 0..64 {
                    let (lx, ly, lz) = (offset[0] as u32 + x, offset[1] as u32 + y, offset[2] as u32 + z);
                    assert_eq!(piece.block_at(x, y, z), large.block_at(lx, ly, lz), "cell {lx} {ly} {lz}");
                }
            }
        }
    }
}

#[test]
fn target_pieces_tile_into_a_two_by_four_grid_with_no_overlap() {
    let pieces: Vec<Structure> =
        SampleBuilder::target_pieces().unwrap().iter().map(|bytes| StructureParser::parse(bytes).unwrap()).collect();
    assert_eq!(pieces.len(), 8);
    for piece in &pieces {
        assert_eq!(piece.size, StructureSize::new(64, 192, 64));
    }
    let entries: Vec<StructureEntry> =
        pieces.iter().enumerate().map(|(index, piece)| EntryFixture::of(index as u64 + 1, piece)).collect();
    let refs: Vec<&StructureEntry> = entries.iter().collect();
    let layout = LayoutSolver::from_origins(&refs).unwrap();
    assert_eq!(layout.size, StructureSize::new(128, 192, 256));

    // piece i = px * 4 + pz sits at (px * 64, 0, pz * 64)
    for PiecePlacement { structure, offset } in &layout.pieces {
        let index = structure.0 as i32 - 1;
        assert_eq!(*offset, [(index / 4) * 64, 0, (index % 4) * 64]);
    }
}

// The whole target in view at the default config: every section gets a slot and none is cut, with the marker on
// the section grid and 8 cells off it.
#[test]
fn the_target_at_the_default_config_is_not_cut() {
    let config = GhostConfig::default();
    for origin in [[0, 64, 0], [8, 64, 8]] {
        let build = TargetBuild::at(origin);
        let mut occupied = Vec::new();
        GhostSections::occupied(&build, config.cells_per_frame(), &mut occupied, &mut Vec::new());
        let centre = [origin[0] + 64, origin[1] + 96, origin[2] + 128];
        let mut wanted = Vec::new();
        GhostSections::wanted(&occupied, GhostView::Whole, centre, 32, &mut wanted);
        assert_eq!(wanted.len(), occupied.len());

        let mut scheduler = GhostScheduler::new(&config);
        assert!(wanted.len() <= scheduler.slot_capacity() as usize, "{} keys at {origin:?}", wanted.len());
        let mut frame = GhostFrame::new();
        loop {
            scheduler.plan(Some(&build), &wanted, &MiniatureGroups::new(), [0, 0, 0], &mut frame);
            assert!(!frame.cut, "cut at {origin:?} with {} slots drawn", frame.draws.len());
            if frame.jobs.is_empty() {
                break;
            }
            let mut report = GhostReport::new();
            for job in &frame.jobs {
                let bytes = TargetBuild::bytes(job);
                report.built.push(SlotBuilt { slot: job.slot, serial: job.serial, generation: 1, bytes, ok: true });
            }
            scheduler.apply_report(&report);
        }

        assert_eq!(frame.draws.len(), wanted.len());
        assert!(scheduler.used_bytes() <= config.mesh_budget_bytes());
        println!(
            "origin {origin:?}: {} keys, {} slots, {} MiB reported",
            wanted.len(),
            scheduler.slot_capacity(),
            scheduler.used_bytes() / (1024 * 1024)
        );
    }
}

#[test]
fn tint_sample_holds_every_leaves_block_and_plants_on_grass() {
    let tint = StructureParser::parse(&SampleBuilder::tint().unwrap()).unwrap();
    let names: Vec<&str> = tint.palette.iter().map(|block| block.name.as_str()).collect();
    for expected in [
        "minecraft:oak_leaves",
        "minecraft:spruce_leaves",
        "minecraft:azalea_leaves_flowered",
        "minecraft:pale_oak_log",
        "minecraft:warped_hyphae",
        "minecraft:bamboo_planks",
        "minecraft:grass_block",
    ] {
        assert!(names.contains(&expected), "missing {expected}");
    }
    let name = |x, y, z| tint.block_at(x, y, z).map(|block| block.name.as_str());
    // the grass row is the fifth row on z, 3 cells per row
    assert_eq!(name(0, 0, 12), Some("minecraft:grass_block"));
    assert_eq!(name(0, 1, 12), Some("minecraft:short_grass"));
    assert_eq!(name(3, 1, 12), None);
    assert_eq!(name(6, 1, 12), Some("minecraft:fern"));
    // the gap between columns stays empty, so every face is drawn
    assert_eq!(name(1, 0, 0), None);
}
