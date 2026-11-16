#![allow(clippy::expect_used)]

use std::collections::HashMap;
use std::hint::black_box;
use std::sync::Arc;

use criterion::{Criterion, criterion_group, criterion_main};
use stratum_core::services::{GhostAssembler, GhostSections, NbtWriter};
use stratum_core::structs::ghost::{GhostJob, PlacedStructure, SectionKey, SectionPart};
use stratum_core::structs::library::{Library, StructureId};
use stratum_core::structs::marker::{Marker, MarkerId, MarkerTarget, MarkerView, Rotation};
use stratum_core::structs::nbt::{NbtCompound, NbtTag};
use stratum_core::structs::structure::{
    BlockEntities, BlockLayer, BlockState, BlockStates, SparseLayer, StateValue, Structure, StructureSize,
};

// Builds the inputs the assembly bench runs on.
struct BenchAssembly;

impl BenchAssembly {
    // Block entity cells in each bench structure; below `BlockEntities::MAX_ENTRIES`.
    const CHESTS: u32 = 4000;

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
        // One chest NBT per cell for the first cells, each naming a different partner, so no two are equal.
        let mut block_entities = BlockEntities::new();
        for cell in 0..Self::CHESTS {
            let nbt = NbtWriter::compound(
                "",
                &[
                    ("id", NbtTag::String("Chest".to_string())),
                    ("pairx", NbtTag::Int(cell as i32)),
                    ("pairz", NbtTag::Int(0)),
                    ("pairlead", NbtTag::Byte(1)),
                ],
            )
            .expect("chest nbt");
            block_entities.insert(cell, nbt);
        }
        Arc::new(Structure {
            size: StructureSize::new(size[0], size[1], size[2]),
            world_origin: None,
            palette,
            primary: BlockLayer::new(cells),
            secondary: SparseLayer::default(),
            block_entities,
        })
    }

    // One section of chests facing north, every cell one half of a pair along X. Each NBT holds its own
    // position, 27 item stacks and the pair keys in world coordinates, as a real double chest does.
    fn paired_chests() -> Arc<Structure> {
        let side = SectionKey::SIZE as u32;
        let (sy, sz) = (side as usize, side as usize);
        let north = StateValue::Text("north".to_string());
        let palette = vec![BlockState::new(
            "minecraft:chest".to_string(),
            BlockStates::new(vec![("minecraft:cardinal_direction".to_string(), north)]),
        )];
        let stack = |slot: i8| {
            NbtTag::Compound(NbtCompound::new(vec![
                ("Count".to_string(), NbtTag::Byte(64)),
                ("Damage".to_string(), NbtTag::Short(0)),
                ("Name".to_string(), NbtTag::String("minecraft:cobblestone".to_string())),
                ("Slot".to_string(), NbtTag::Byte(slot)),
                ("WasPickedUp".to_string(), NbtTag::Byte(0)),
            ]))
        };
        let mut block_entities = BlockEntities::new();
        for x in 0..side as usize {
            for y in 0..sy {
                for z in 0..sz {
                    let partner = if x % 2 == 0 { x + 1 } else { x - 1 };
                    let nbt = NbtWriter::compound(
                        "",
                        &[
                            ("id", NbtTag::String("Chest".to_string())),
                            ("x", NbtTag::Int(x as i32)),
                            ("y", NbtTag::Int(y as i32)),
                            ("z", NbtTag::Int(z as i32)),
                            ("Items", NbtTag::List((0..27).map(stack).collect())),
                            ("pairlead", NbtTag::Byte(i8::from(x % 2 == 0))),
                            ("pairx", NbtTag::Int(partner as i32)),
                            ("pairz", NbtTag::Int(z as i32)),
                        ],
                    )
                    .expect("chest nbt");
                    block_entities.insert(((x * sy + y) * sz + z) as u32, nbt);
                }
            }
        }
        Arc::new(Structure {
            size: StructureSize::new(side, side, side),
            world_origin: None,
            palette,
            primary: BlockLayer::new(vec![0; (side * side * side) as usize]),
            secondary: SparseLayer::default(),
            block_entities,
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

// `fill_job_*` is the game thread filling the job of one Whole section that is all paired chests: the
// cells, and for each chest its pair decision and table entry.
fn bench_fill_job(c: &mut Criterion) {
    let mut group = c.benchmark_group("ghost_assembler");
    let library = Library::new();
    let loaded: HashMap<StructureId, Arc<Structure>> = [(StructureId(1), BenchAssembly::paired_chests())].into();
    let mut marker = BenchAssembly::marker(Rotation::R0);
    marker.origin = [0, 64, 0];
    let build = GhostAssembler::assemble(&marker, &library, &loaded, 1).expect("assembles");
    build.block_entities();
    let key = SectionKey { x: 0, y: 4, z: 0, layer: None, part: SectionPart::Whole };
    let mut job = GhostJob::new(key);
    GhostSections::fill_job(&build, key, &mut job);
    assert_eq!(job.block_entities.len(), 2048, "one lead per pair");
    group.bench_function("fill_job_paired_chests_16x16x16", |b| {
        b.iter(|| GhostSections::fill_job(black_box(&build), key, &mut job))
    });
    group.finish();
}

criterion_group!(benches, bench_assemble, bench_fill_job);
criterion_main!(benches);
