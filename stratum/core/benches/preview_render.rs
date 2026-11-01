#![allow(clippy::expect_used)]

use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use criterion::{Criterion, criterion_group, criterion_main};
use stratum_core::config::PreviewConfig;
use stratum_core::services::StructureParser;
use stratum_core::services::preview::{PreviewMesher, PreviewRasterizer, TexturePack};
use stratum_core::structs::preview::{BlockTexture, OrbitCamera, PreviewMesh, PreviewQuad, PreviewStop};
use stratum_core::structs::structure::StructureSize;

// Reads the inputs: the game's resource packs from `STRATUM_PREVIEW_PACKS` (no textures when unset) and the
// structure files from `STRATUM_PREVIEW_FILES` (`;` between paths), or the repo's two samples.
struct BenchInputs;

impl BenchInputs {
    // Quads one `push` bench iteration adds.
    const PUSH_QUADS: usize = 100_000;

    fn pack() -> TexturePack {
        let Ok(dir) = std::env::var("STRATUM_PREVIEW_PACKS") else {
            println!("STRATUM_PREVIEW_PACKS unset, rendering with no textures");
            return TexturePack::empty(PreviewConfig::default().texture_cache());
        };
        let started = Instant::now();
        let pack = TexturePack::load(Path::new(&dir), PreviewConfig::default().texture_cache());
        println!(
            "texture pack load: {:.1} ms, {} packs, {} files, {} bytes",
            started.elapsed().as_secs_f64() * 1000.0,
            pack.pack_count(),
            pack.file_count(),
            pack.archive_bytes()
        );
        pack
    }

    fn files() -> Vec<PathBuf> {
        match std::env::var("STRATUM_PREVIEW_FILES") {
            Ok(list) => list.split(';').filter(|part| !part.is_empty()).map(PathBuf::from).collect(),
            Err(_) => {
                let samples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples");
                vec![samples.join("tint.mcstructure"), samples.join("easy-place-full.mcstructure")]
            }
        }
    }

    // A quad of a 1 block face whose corners move with `index`, so the bounds grow on every axis.
    fn quad(index: usize) -> PreviewQuad {
        let at = (index % 2000) as i16 * 16;
        PreviewQuad {
            corners: [[at, 0, at], [at + 16, 0, at], [at + 16, 16, at], [at, 16, at]],
            uvs: [[0, 0], [16, 0], [16, 16], [0, 16]],
            texture: 0,
            shade: 255,
            normal: [0, 0, 1],
        }
    }
}

// The mesh build, one 640 by 480 render from the start view, and the render with the hand-off of its picture to
// the UI, per structure.
fn bench_preview(c: &mut Criterion) {
    let mut group = c.benchmark_group("preview_render");
    group.sample_size(20);
    let mut pack = BenchInputs::pack();
    for path in BenchInputs::files() {
        let name = path.file_stem().and_then(|stem| stem.to_str()).unwrap_or("structure").to_owned();
        let bytes = std::fs::read(&path).expect("structure file reads");
        let structure = StructureParser::parse(&bytes).expect("structure parses");
        let mesher = PreviewMesher::new(PreviewConfig::default().max_quads(), PreviewStop::new());
        let mesh = mesher.build(&structure, &mut pack).expect("mesh builds");
        let mut rasterizer = PreviewRasterizer::new(PreviewStop::new());
        let triangles = rasterizer.render(&mesh, OrbitCamera::start(), 640, 480);
        println!(
            "{name}: size {}x{}x{}, {} quads, {} triangles drawn, {} missing blocks",
            mesh.size.x,
            mesh.size.y,
            mesh.size.z,
            mesh.quads().len(),
            triangles,
            mesh.missing
        );
        group.bench_function(format!("mesh_{name}"), |b| {
            b.iter(|| mesher.build(black_box(&structure), &mut pack).expect("mesh builds"))
        });
        group.bench_function(format!("render_640x480_{name}"), |b| {
            b.iter(|| rasterizer.render(black_box(&mesh), OrbitCamera::start(), 640, 480))
        });
        // the worker swaps the picture out for the buffer the UI gave back, as the service does
        let mut spare = Vec::new();
        group.bench_function(format!("picture_640x480_{name}"), |b| {
            b.iter(|| {
                rasterizer.render(black_box(&mesh), OrbitCamera::start(), 640, 480);
                let picture = rasterizer.swap_pixels(std::mem::take(&mut spare));
                spare = black_box(picture);
            })
        });
    }
    group.finish();
}

// `PreviewMesh::push`, which grows the bounds with each quad, against a plain `Vec` push as a floor for the cost
// of the bounds.
fn bench_push(c: &mut Criterion) {
    let mut group = c.benchmark_group("preview_mesh");
    let quads: Vec<PreviewQuad> = (0..BenchInputs::PUSH_QUADS).map(BenchInputs::quad).collect();
    let textures = vec![Arc::new(BlockTexture::solid([160, 160, 160, 255]))];
    group.bench_function("push_100k", |b| {
        b.iter(|| {
            let mut mesh =
                PreviewMesh::new(StructureSize::new(2000, 1, 2000), textures.clone(), BenchInputs::PUSH_QUADS);
            for quad in &quads {
                mesh.push(*quad);
            }
            black_box(mesh.bounds())
        })
    });
    group.bench_function("vec_push_100k", |b| {
        b.iter(|| {
            let mut plain: Vec<PreviewQuad> = Vec::new();
            for quad in &quads {
                plain.push(*quad);
            }
            black_box(plain.len())
        })
    });
    group.finish();
}

criterion_group!(benches, bench_preview, bench_push);
criterion_main!(benches);
