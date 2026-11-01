#![allow(clippy::expect_used)]

use std::hint::black_box;
use std::sync::Arc;

use criterion::{Criterion, criterion_group, criterion_main};
use stratum_core::services::{GhostAssembler, GhostRay, ItemRules, PlaceNeeds};
use stratum_core::structs::easy_place::RayLimits;
use stratum_core::structs::ghost::{GhostBuild, GhostView, PlacedStructure};
use stratum_core::structs::marker::Rotation;
use stratum_core::structs::structure::{BlockLayer, BlockState, BlockStates, SparseLayer, Structure, StructureSize};

// Builds the inputs the ray bench runs on.
struct BenchRay;

impl BenchRay {
    // A build of water, which is never a target, with stone in the last X slice only. A ray along X
    // from the near side walks every cell within its reach and hits nothing.
    fn build(size: [u32; 3]) -> GhostBuild {
        let (sy, sz) = (size[1] as usize, size[2] as usize);
        let mut cells = vec![1_u16; size[0] as usize * sy * sz];
        let last = (size[0] as usize - 1) * sy * sz;
        cells[last..].fill(0);
        let structure = Arc::new(Structure {
            size: StructureSize::new(size[0], size[1], size[2]),
            world_origin: None,
            palette: ["stone", "water"]
                .iter()
                .map(|name| BlockState::new(format!("minecraft:{name}"), BlockStates::default()))
                .collect(),
            primary: BlockLayer::new(cells),
            secondary: SparseLayer::default(),
        });
        let placed = [PlacedStructure { structure: Arc::clone(&structure), origin: [0, 0, 0] }];
        GhostAssembler::build(&placed, &[structure], [0, 0, 0], Rotation::R0, 1, None).expect("builds")
    }
}

// The ray runs along X through the build at the mid height, so every step reads a water cell.
fn bench_ray(c: &mut Criterion) {
    let mut group = c.benchmark_group("ghost_ray");
    let build = BenchRay::build([128, 64, 128]);
    let needs = PlaceNeeds::new(&build.palette, &ItemRules::new());
    let eye = [-1.0, 32.5, 64.5];
    let dir = [1.0, 0.0, 0.0];
    for reach in [5.0_f64, 64.0] {
        group.bench_function(format!("first_spot_reach_{reach}"), |b| {
            b.iter(|| {
                GhostRay::first_spot(
                    black_box(&build),
                    GhostView::Whole,
                    &needs,
                    black_box(eye),
                    black_box(dir),
                    &RayLimits::reach(reach),
                )
            })
        });
    }
    // A stop cell and a body box that the ray never reaches, so the walk tests both on every cell.
    let limits = RayLimits::new(64.0, Some([0, 0, 0]), false, Some(([0, 0, 0], [0, 1, 0])));
    group.bench_function("first_spot_reach_64_with_stop_and_body", |b| {
        b.iter(|| {
            GhostRay::first_spot(black_box(&build), GhostView::Whole, &needs, black_box(eye), black_box(dir), &limits)
        })
    });
    group.finish();
}

criterion_group!(benches, bench_ray);
criterion_main!(benches);
