#![allow(clippy::expect_used)]

use std::hint::black_box;
use std::sync::Arc;

use criterion::{Criterion, criterion_group, criterion_main};
use stratum_core::services::BuildOrder;
use stratum_core::structs::ghost::GhostPiece;
use stratum_core::structs::structure::{BlockLayer, BlockState, BlockStates, SparseLayer, Structure, StructureSize};

// Builds the inputs the build order bench runs on.
struct BenchOrder;

impl BenchOrder {
    // One layer of stone with every tenth block air, spread evenly.
    fn piece(side: u32) -> GhostPiece {
        let cells: Vec<u16> = (0..side * side).map(|cell| if cell % 10 == 3 { 1 } else { 0 }).collect();
        let structure = Structure {
            size: StructureSize::new(side, 1, side),
            world_origin: None,
            palette: ["stone", "air"]
                .iter()
                .map(|name| BlockState::new(format!("minecraft:{name}"), BlockStates::default()))
                .collect(),
            primary: BlockLayer::new(cells),
            secondary: SparseLayer::default(),
            block_entities: Default::default(),
        };
        GhostPiece { structure: Arc::new(structure), world_min: [0, 0, 0], remap: Vec::new() }
    }
}

fn bench_layer(c: &mut Criterion) {
    let mut group = c.benchmark_group("build_order");
    let piece = BenchOrder::piece(256);
    group.bench_function("layer_256x256_10_percent_air", |b| {
        b.iter(|| BuildOrder::layer(black_box(&piece), black_box(0), black_box(&[])))
    });
    group.finish();
}

criterion_group!(benches, bench_layer);
criterion_main!(benches);
