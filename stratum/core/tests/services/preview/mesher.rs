use std::time::{Duration, Instant};

use stratum_core::config::PreviewConfig;
use stratum_core::errors::PreviewError;
use stratum_core::services::preview::PreviewMesher;
use stratum_core::structs::preview::{PreviewBounds, PreviewMesh, PreviewStop};
use stratum_core::structs::structure::{StateValue, StructureSize};

use crate::support::{BlockFactory, PreviewFixture};

// Meshes of small structures of plain blocks.
struct Meshes;

impl Meshes {
    fn of(size: [u32; 3], blocks: &[([u32; 3], &str)]) -> PreviewMesh {
        let blocks: Vec<_> = blocks.iter().map(|(at, name)| (*at, BlockFactory::plain(name))).collect();
        PreviewFixture::mesh(&PreviewFixture::structure(size, &blocks))
    }
}

#[test]
fn two_touching_stone_cubes_hide_the_faces_between_them() {
    let mesh = Meshes::of([2, 1, 1], &[([0, 0, 0], "stone"), ([1, 0, 0], "stone")]);
    assert_eq!(mesh.quads().len(), 10);
}

#[test]
fn air_and_structure_void_draw_nothing_and_hide_nothing() {
    let mesh = Meshes::of([3, 1, 1], &[([0, 0, 0], "stone"), ([1, 0, 0], "air"), ([2, 0, 0], "structure_void")]);
    assert_eq!(mesh.quads().len(), 6);
}

#[test]
fn leaves_keep_the_faces_between_them_and_glass_does_not() {
    let leaves = Meshes::of([2, 1, 1], &[([0, 0, 0], "oak_leaves"), ([1, 0, 0], "oak_leaves")]);
    assert_eq!(leaves.quads().len(), 12);
    let glass = Meshes::of([2, 1, 1], &[([0, 0, 0], "glass"), ([1, 0, 0], "glass")]);
    assert_eq!(glass.quads().len(), 10);
    // stone beside glass keeps its face: glass hides only glass
    let mixed = Meshes::of([2, 1, 1], &[([0, 0, 0], "stone"), ([1, 0, 0], "glass")]);
    assert_eq!(mixed.quads().len(), 11);
}

#[test]
fn a_bottom_slab_under_stone_keeps_its_top_face() {
    let slab = BlockFactory::block("oak_slab", &[("minecraft:vertical_half", StateValue::Text("bottom".into()))]);
    let structure =
        PreviewFixture::structure([1, 2, 1], &[([0, 0, 0], slab), ([0, 1, 0], BlockFactory::plain("stone"))]);
    let mesh = PreviewFixture::mesh(&structure);
    // the slab's top is at half height, so the stone above does not touch it; the stone's bottom shows too
    assert_eq!(mesh.quads().len(), 12);
    let top = mesh.quads().iter().filter(|quad| quad.normal == [0, 1, 0]).map(|quad| quad.corners[0][1]).min();
    assert_eq!(top, Some(8));
}

#[test]
fn stairs_draw_a_slab_and_a_step_on_the_high_side() {
    let stairs = BlockFactory::block(
        "oak_stairs",
        &[("upside_down_bit", StateValue::Int(0)), ("weirdo_direction", StateValue::Int(0))],
    );
    let mesh = PreviewFixture::mesh(&PreviewFixture::structure([1, 1, 1], &[([0, 0, 0], stairs)]));
    // six slab faces and five step faces: the step's bottom sits on the slab
    assert_eq!(mesh.quads().len(), 11);
    let step_top = mesh.quads().iter().find(|quad| quad.normal == [0, 1, 0] && quad.corners[0][1] == 16).unwrap();
    assert!(step_top.corners.iter().all(|corner| corner[0] >= 8));
}

#[test]
fn a_cross_plant_is_two_quads_seen_from_both_sides() {
    let mesh = Meshes::of([1, 1, 1], &[([0, 0, 0], "short_grass")]);
    assert_eq!(mesh.quads().len(), 2);
    assert!(mesh.quads().iter().all(|quad| quad.normal == [0, 0, 0]));
}

#[test]
fn quads_sit_at_their_cell_in_sixteenths() {
    let mesh = Meshes::of([3, 2, 2], &[([2, 1, 1], "stone")]);
    let lowest = mesh.quads().iter().flat_map(|quad| quad.corners).min().unwrap();
    assert_eq!(lowest, [32, 16, 16]);
}

#[test]
fn blocks_with_no_texture_use_the_stand_in_and_are_counted() {
    let mesh = Meshes::of([2, 1, 1], &[([0, 0, 0], "stone"), ([1, 0, 0], "dirt")]);
    assert_eq!(mesh.missing, 2);
    assert!(mesh.quads().iter().all(|quad| quad.texture == 0));
}

#[test]
fn a_side_past_the_corner_range_is_refused() {
    let mut structure = PreviewFixture::structure([1, 1, 1], &[([0, 0, 0], BlockFactory::plain("stone"))]);
    structure.size = StructureSize::new(PreviewMesher::MAX_SIDE + 1, 1, 1);
    let result = PreviewFixture::mesher().build(&structure, &mut PreviewFixture::no_textures());
    assert_eq!(result, Err(PreviewError::TooLarge));
}

#[test]
fn the_bounds_hug_the_drawn_blocks_in_sixteenths() {
    let mesh = Meshes::of([3, 8, 3], &[([1, 0, 1], "stone"), ([1, 2, 1], "stone")]);
    let mut want = PreviewBounds::empty();
    want.include([16, 0, 16]);
    want.include([32, 48, 32]);
    assert_eq!(mesh.bounds(), want);
}

#[test]
fn a_mesh_with_no_quads_has_empty_bounds() {
    let mesh = Meshes::of([2, 2, 2], &[([0, 0, 0], "air")]);
    assert!(mesh.bounds().is_empty());
}

#[test]
fn the_quad_bound_comes_from_the_config() {
    // three cubes apart: 18 quads, against a bound of 20 that leaves no room for a third block of 12
    let config = PreviewConfig { max_quads: 20, ..PreviewConfig::default() };
    let blocks: Vec<_> = [0u32, 2, 4].iter().map(|x| ([*x, 0, 0], BlockFactory::plain("stone"))).collect();
    let structure = PreviewFixture::structure([5, 1, 1], &blocks);
    let mesher = PreviewMesher::new(config.max_quads(), PreviewStop::new());
    let mesh = mesher.build(&structure, &mut PreviewFixture::no_textures()).unwrap();
    assert!(mesh.truncated);
    assert_eq!(mesh.quads().len(), 12);
}

#[test]
fn a_build_with_the_stop_set_ends_as_stopped() {
    let stop = PreviewStop::new();
    stop.stop();
    let structure = PreviewFixture::structure([1, 1, 1], &[([0, 0, 0], BlockFactory::plain("stone"))]);
    let result = PreviewMesher::new(PreviewConfig::default().max_quads(), stop)
        .build(&structure, &mut PreviewFixture::no_textures());
    assert_eq!(result, Err(PreviewError::Stopped));
}

#[test]
fn a_stop_during_a_long_build_ends_it_within_the_budget() {
    let budget = Duration::from_millis(300);
    // every cell a glass block: glass hides glass, so few quads, but every cell is visited
    let structure = PreviewFixture::solid([192, 192, 192], BlockFactory::plain("glass"));
    let stop = PreviewStop::new();
    let mesher = PreviewMesher::new(PreviewConfig::default().max_quads(), stop.clone());
    let (done_tx, done_rx) = std::sync::mpsc::channel();
    let builder = std::thread::spawn(move || {
        let started = Instant::now();
        let result = mesher.build(&structure, &mut PreviewFixture::no_textures());
        done_tx.send(started.elapsed()).unwrap();
        result
    });
    // the whole build takes about 2 s on the test profile
    std::thread::sleep(Duration::from_millis(20));
    let stopped_at = Instant::now();
    stop.stop();
    let ran = done_rx.recv_timeout(budget).expect("the build ends within the budget after the stop");
    assert!(stopped_at.elapsed() < budget);
    assert_eq!(builder.join().unwrap(), Err(PreviewError::Stopped), "the build ran {ran:?} and was not stopped");
}
