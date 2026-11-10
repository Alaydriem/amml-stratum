#![allow(clippy::expect_used)]

use std::collections::HashMap;
use std::hint::black_box;
use std::sync::Arc;

use criterion::{Criterion, criterion_group, criterion_main};
use stratum_core::services::GhostAssembler;
use stratum_core::structs::ghost::PlacedStructure;
use stratum_core::structs::library::{Library, StructureId};
use stratum_core::structs::marker::{Marker, MarkerId, MarkerTarget, MarkerView, Rotation};
use stratum_core::structs::structure::{BlockLayer, BlockState, BlockStates, SparseLayer, Structure, StructureSize};

// Builds the inputs the assembly bench runs on.
struct BenchAssembly;

impl BenchAssembly {
    // Stone and air in alternating 8-block layers along Y, with a few block kinds.
    fn structure(size: [u32; 3]) -> Arc<Structure> {
        let (sy, sz) = (size[1] as usize, size[2] as usize);
        let mut cells = vec![BlockLayer::NONE; size[0] as usize * sy * sz];
        for x in 0..size[0] as usize {
            for y in 0..sy {
                if (y / 8) % 2 == 0 {
                    let start = (x * sy + y) * sz;
                    cells[start..start + sz].fill((x % 4) as u16);
                }
            }
        }
        let palette = ["stone", "dirt", "oak_planks", "glass"]
            .iter()
            .map(|name| BlockState::new(format!("minecraft:{name}"), BlockStates::default()))
            .collect();
        Arc::new(Structure {
            size: StructureSize::new(size[0], size[1], size[2]),
            world_origin: None,
            palette,
            primary: BlockLayer::new(cells),
            secondary: SparseLayer::default(),
        })
    }

    fn marker(rotation: Rotation) -> Marker {
        Marker {
            id: MarkerId(1),
            name: "bench".to_string(),
            origin: [0, 64, 0],
            rotation,
            target: MarkerTarget::Structure(StructureId(1)),
            view: MarkerView::default(),
        }
    }
}

// `assemble_*` is the whole assembly, turn included, in one call. `press_*` is what the game thread
// runs on a press of the same structures: `build` with the palette and remaps of the current build,
// and for a turn the copies `worker_turn_*` made on the worker thread.
fn bench_assemble(c: &mut Criterion) {
    let mut group = c.benchmark_group("ghost_assembler");
    group.sample_size(10);
    let library = Library::new();
    for (name, size) in [("256x128x256", [256, 128, 256]), ("256x384x256_max_volume", [256, 384, 256])] {
        let source = BenchAssembly::structure(size);
        let loaded: HashMap<StructureId, Arc<Structure>> = [(StructureId(1), Arc::clone(&source))].into();
        for (label, rotation) in [("r0", Rotation::R0), ("r90", Rotation::R90)] {
            let marker = BenchAssembly::marker(rotation);
            group.bench_function(format!("assemble_{name}_{label}"), |b| {
                b.iter(|| GhostAssembler::assemble(black_box(&marker), &library, &loaded, 1).expect("assembles"))
            });
        }
        let current =
            GhostAssembler::assemble(&BenchAssembly::marker(Rotation::R0), &library, &loaded, 1).expect("assembles");
        let placed = vec![PlacedStructure { structure: Arc::clone(&source), origin: [1, 64, 0] }];
        let sources = vec![Arc::clone(&source)];
        group.bench_function(format!("press_move_{name}_r0"), |b| {
            b.iter(|| {
                GhostAssembler::build(black_box(&placed), &sources, [1, 64, 0], Rotation::R0, 1, Some(&current))
                    .expect("builds")
            })
        });
        group.bench_function(format!("worker_turn_{name}_r90"), |b| {
            b.iter(|| GhostAssembler::turn(black_box(&source), Rotation::R90))
        });
        let turned = vec![Arc::new(GhostAssembler::turn(&source, Rotation::R90))];
        group.bench_function(format!("press_turn_{name}_r90"), |b| {
            b.iter(|| {
                GhostAssembler::build(black_box(&placed), &turned, [1, 64, 0], Rotation::R90, 1, Some(&current))
                    .expect("builds")
            })
        });
    }
    group.finish();
}

criterion_group!(benches, bench_assemble);
criterion_main!(benches);
