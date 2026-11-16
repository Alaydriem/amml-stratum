use std::sync::atomic::Ordering;

use alaydriem_stratum::ghost::GhostRenderer;
use stratum_core::structs::ghost::GhostJob;

use crate::support::fake_ghost::{
    BIOME_NULL, BIOME_QUERIES, CHUNK_NULL, CHUNK_QUERIES, ENDED_COLORS, ENDED_EXTRA, ENDED_POSITIONS, EXACT_AS,
    EXTRA_FULL, EXTRA_SHORT, EndedExtra, FILL_EXTRA, GRAPHICS_SHAPE, POLICY_CALLS, POLICY_COLOUR, WATER_ARGB,
    WATER_COLOR_CALLS,
};
use crate::support::{FakeGhost, GhostRig};

const WATER: &str = "minecraft:water";
const LEAVES: &str = "minecraft:leaves";

// one frame of `job` of the block `name` built in a fake region on a rig whose game calls `calls` changed; the rgb of
// the first vertex of the mesh the job made
fn build_with(name: &'static str, job: GhostJob, calls: impl FnOnce(&mut GhostRig)) -> [u8; 3] {
    *EXACT_AS.lock().unwrap() = Some(name);
    let mut rig = GhostRig::new();
    calls(&mut rig);
    rig.renderer = GhostRenderer::new(&rig.calls);
    let mut frame = GhostRig::ghost_frame(1, 1, &[name]);
    frame.jobs = vec![job];
    rig.publish(&frame);
    rig.frame(Some(FakeGhost::region()));
    let colors = ENDED_COLORS.lock().unwrap();
    let mesh = colors.iter().rev().find(|colors| !colors.is_empty()).expect("the job made a mesh");
    let [r, g, b, _] = mesh[0].to_le_bytes();
    [r, g, b]
}

fn build_water(job: GhostJob) -> [u8; 3] {
    build_with(WATER, job, |_| {})
}

#[test]
fn water_takes_the_biome_water_colour() {
    WATER_ARGB.store(0xFF40_80FF, Ordering::SeqCst);

    let rgb = build_water(GhostRig::job(0, 1, 0, [0, 64, 0]));

    // the grey 0x80 of the fake tessellator times 64/255, 128/255 and 1
    assert_eq!(rgb, [32, 64, 128]);
}

#[test]
fn a_zero_water_colour_leaves_the_vertices() {
    WATER_ARGB.store(0xFF00_0000, Ordering::SeqCst);

    let rgb = build_water(GhostRig::job(0, 1, 0, [0, 64, 0]));

    assert_eq!(rgb, [0x80, 0x80, 0x80]);
}

#[test]
fn a_missing_chunk_leaves_the_vertices_and_asks_no_biome() {
    WATER_ARGB.store(0xFF40_80FF, Ordering::SeqCst);
    CHUNK_NULL.store(true, Ordering::SeqCst);

    let rgb = build_water(GhostRig::job(0, 1, 0, [0, 64, 0]));

    assert_eq!(rgb, [0x80, 0x80, 0x80]);
    assert!(BIOME_QUERIES.lock().unwrap().is_empty());
}

#[test]
fn a_missing_biome_leaves_the_vertices_and_asks_no_colour() {
    WATER_ARGB.store(0xFF40_80FF, Ordering::SeqCst);
    BIOME_NULL.store(true, Ordering::SeqCst);

    let rgb = build_water(GhostRig::job(0, 1, 0, [0, 64, 0]));

    assert_eq!(rgb, [0x80, 0x80, 0x80]);
    assert_eq!(WATER_COLOR_CALLS.load(Ordering::SeqCst), 0);
}

#[test]
fn a_block_at_negative_coordinates_asks_its_own_chunk_and_column() {
    WATER_ARGB.store(0xFF40_80FF, Ordering::SeqCst);

    build_water(GhostRig::job(0, 1, 0, [-1, 64, -17]));

    assert_eq!(CHUNK_QUERIES.lock().unwrap().last().copied(), Some((-1, -2)));
    // x 15, z 15 of the chunk, y 64: x | z << 8 | y << 16
    assert_eq!(BIOME_QUERIES.lock().unwrap().last().copied(), Some(15 | 15 << 8 | 64 << 16));
}

#[test]
fn water_with_the_water_tint_entries_off_keeps_its_vertices() {
    WATER_ARGB.store(0xFF40_80FF, Ordering::SeqCst);

    let rgb = build_with(WATER, GhostRig::job(0, 1, 0, [0, 64, 0]), |rig| {
        rig.calls.level_chunk_get_biome = 0;
        rig.calls.biome_get_water_color = 0;
        rig.calls.get_chunk_slot = 0;
    });

    assert_eq!(rgb, [0x80, 0x80, 0x80]);
    assert!(CHUNK_QUERIES.lock().unwrap().is_empty());
}

#[test]
fn a_policy_tinted_block_takes_the_policy_colour() {
    *POLICY_COLOUR.lock().unwrap() = [0.5, 0.25, 1.0, 0.0];

    let rgb = build_with(LEAVES, GhostRig::job(0, 1, 0, [0, 64, 0]), |_| {});

    // the grey 0x80 times 0.5, 0.25 and 1
    assert_eq!(rgb, [64, 32, 128]);
    assert_eq!(POLICY_CALLS.load(Ordering::SeqCst), 1);
    assert!(CHUNK_QUERIES.lock().unwrap().is_empty(), "a policy block does not ask for the water colour");
}

#[test]
fn a_zero_policy_colour_leaves_the_vertices() {
    *POLICY_COLOUR.lock().unwrap() = [0.0, 0.0, 0.0, 0.0];

    let rgb = build_with(LEAVES, GhostRig::job(0, 1, 0, [0, 64, 0]), |_| {});

    assert_eq!(rgb, [0x80, 0x80, 0x80]);
    assert_eq!(POLICY_CALLS.load(Ordering::SeqCst), 1);
}

const CROSS_TEXTURE: i32 = 1;
const CHAIN: i32 = 123;
const DOUBLESIDED_CROSS_TEXTURE: i32 = 90;

// the positions and colours of the mesh one cell of a block of `shape` made
fn mesh_of_shape(shape: i32) -> (Vec<[f32; 3]>, Vec<u32>) {
    GRAPHICS_SHAPE.store(shape, Ordering::SeqCst);
    build_with("minecraft:stone", GhostRig::job(0, 1, 0, [0, 64, 0]), |_| {});
    let positions = ENDED_POSITIONS.lock().unwrap().iter().rev().find(|mesh| !mesh.is_empty()).cloned();
    let colors = ENDED_COLORS.lock().unwrap().iter().rev().find(|mesh| !mesh.is_empty()).cloned();
    (positions.expect("the job made a mesh"), colors.expect("the job made a mesh"))
}

#[test]
fn a_double_sided_cell_emits_both_windings() {
    for shape in [CROSS_TEXTURE, CHAIN] {
        let (positions, colors) = mesh_of_shape(shape);

        assert_eq!(positions.len(), 8, "shape {shape}: the quad and its reversed copy");
        let reversed: Vec<[f32; 3]> = positions[..4].iter().rev().copied().collect();
        assert_eq!(positions[4..], reversed[..], "shape {shape}: the copy has the opposite vertex order");
        assert_eq!(colors[4..], colors[..4], "shape {shape}: the copy has the same colour and alpha");
    }
}

#[test]
fn a_full_cube_and_an_already_double_sided_shape_are_not_doubled() {
    for shape in [0, DOUBLESIDED_CROSS_TEXTURE] {
        let (positions, colors) = mesh_of_shape(shape);

        assert_eq!(positions.len(), 4, "shape {shape}");
        assert_eq!(colors.len(), 4, "shape {shape}");
    }
}

// the normals, tangents and first UV set of the mesh one cell of a chain made
fn extra_of_chain(fill: u32) -> (usize, EndedExtra) {
    FILL_EXTRA.store(fill, Ordering::SeqCst);
    let (positions, _) = mesh_of_shape(CHAIN);
    let (normals, tangents, uvs) = ENDED_EXTRA
        .lock()
        .unwrap()
        .iter()
        .rev()
        .find(|mesh| !mesh.2.is_empty() || !mesh.0.is_empty())
        .cloned()
        .unwrap_or_default();
    (positions.len(), (normals, tangents, uvs))
}

#[test]
fn every_per_vertex_vector_is_mirrored_with_its_vertex() {
    let (positions, (normals, tangents, uvs)) = extra_of_chain(EXTRA_FULL);

    assert_eq!(positions, 8);
    assert_eq!((normals.len(), tangents.len(), uvs.len()), (8, 8, 8), "each vector has one entry per vertex");
    assert!(normals[4..].iter().all(|normal| *normal == [0.0, -1.0, 0.0, 0.0]), "the copy faces the other way");
    assert!(tangents[4..].iter().all(|tangent| *tangent == [-1.0, 0.0, 0.0, 1.0]));
    let reversed: Vec<[f32; 2]> = uvs[..4].iter().rev().copied().collect();
    assert_eq!(uvs[4..], reversed[..], "each UV moves with its vertex");
}

#[test]
fn a_vector_with_a_count_that_does_not_fit_leaves_the_cell_one_sided() {
    let (positions, (normals, tangents, _)) = extra_of_chain(EXTRA_SHORT);

    assert_eq!(positions, 4);
    assert_eq!((normals.len(), tangents.len()), (2, 0), "nothing was written to any vector");
}
