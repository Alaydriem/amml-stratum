use stratum_core::config::PreviewConfig;
use stratum_core::services::preview::PreviewRasterizer;
use stratum_core::structs::preview::{OrbitCamera, PreviewMesh, PreviewStop};

use crate::support::{BlockFactory, PreviewFixture};

// A stone cube to draw, and readers of the picture.
struct Pictures;

impl Pictures {
    fn cube() -> PreviewMesh {
        let structure = PreviewFixture::structure([1, 1, 1], &[([0, 0, 0], BlockFactory::plain("stone"))]);
        PreviewFixture::mesh(&structure)
    }

    fn empty() -> PreviewMesh {
        let cube = Self::cube();
        PreviewMesh::new(cube.size, cube.textures, PreviewConfig::default().max_quads())
    }

    fn rasterizer() -> PreviewRasterizer {
        PreviewRasterizer::new(PreviewStop::new())
    }

    fn pixel(rasterizer: &PreviewRasterizer, x: u32, y: u32) -> [u8; 4] {
        let at = ((y * rasterizer.width() + x) * 4) as usize;
        rasterizer.pixels()[at..at + 4].try_into().unwrap()
    }

    fn covered(rasterizer: &PreviewRasterizer) -> usize {
        rasterizer.pixels().as_chunks::<4>().0.iter().filter(|px| px[3] != 0).count()
    }

    // The left, top, right and bottom pixel that is drawn.
    fn drawn_box(rasterizer: &PreviewRasterizer) -> [u32; 4] {
        let (width, height) = (rasterizer.width(), rasterizer.height());
        let mut found = [u32::MAX, u32::MAX, 0, 0];
        for y in 0..height {
            for x in 0..width {
                if Self::pixel(rasterizer, x, y)[3] != 0 {
                    found = [found[0].min(x), found[1].min(y), found[2].max(x), found[3].max(y)];
                }
            }
        }
        found
    }
}

#[test]
fn a_cube_fills_the_middle_and_leaves_the_corners_clear() {
    let mut rasterizer = Pictures::rasterizer();
    let drawn = rasterizer.render(&Pictures::cube(), OrbitCamera::start(), 64, 48);
    // three faces turn towards the camera, two triangles each
    assert_eq!(drawn, 6);
    assert_eq!(Pictures::pixel(&rasterizer, 32, 24)[3], 255);
    assert_eq!(Pictures::pixel(&rasterizer, 0, 0), [0, 0, 0, 0]);
}

#[test]
fn the_top_face_is_brighter_than_the_sides() {
    let mut rasterizer = Pictures::rasterizer();
    rasterizer.render(&Pictures::cube(), OrbitCamera::new(45.0, 30.0), 200, 200);
    // the stand-in texture is one grey; the top shows just above the middle, the sides below it
    let top = Pictures::pixel(&rasterizer, 100, 70);
    let side = Pictures::pixel(&rasterizer, 100, 140);
    assert_eq!(top[3], 255);
    assert_eq!(side[3], 255);
    assert!(top[0] > side[0], "top {top:?} side {side:?}");
}

#[test]
fn the_image_is_capped_at_its_limits() {
    let mut rasterizer = Pictures::rasterizer();
    rasterizer.render(&Pictures::cube(), OrbitCamera::start(), 4000, 3000);
    assert_eq!(
        (rasterizer.width(), rasterizer.height()),
        (PreviewRasterizer::MAX_WIDTH, PreviewRasterizer::MAX_HEIGHT)
    );
    assert_eq!(rasterizer.pixels().len(), (PreviewRasterizer::MAX_WIDTH * PreviewRasterizer::MAX_HEIGHT * 4) as usize);
}

#[test]
fn a_render_clears_the_last_picture() {
    let mut rasterizer = Pictures::rasterizer();
    rasterizer.render(&Pictures::cube(), OrbitCamera::start(), 64, 48);
    let empty = Pictures::empty();
    rasterizer.render(&empty, OrbitCamera::start(), 64, 48);
    assert!(rasterizer.pixels().iter().all(|byte| *byte == 0));
}

#[test]
fn zoom_scales_the_picture_about_its_centre() {
    let mut rasterizer = Pictures::rasterizer();
    rasterizer.render(&Pictures::cube(), OrbitCamera::start(), 64, 64);
    let fitted = Pictures::covered(&rasterizer);
    rasterizer.render(&Pictures::cube(), OrbitCamera::start().zoomed(-5), 64, 64);
    let out = Pictures::covered(&rasterizer);
    assert_eq!(Pictures::pixel(&rasterizer, 32, 32)[3], 255);
    rasterizer.render(&Pictures::cube(), OrbitCamera::start().zoomed(5), 64, 64);
    let close = Pictures::covered(&rasterizer);
    assert!(out < fitted && fitted < close, "out {out} fitted {fitted} close {close}");
}

#[test]
fn the_start_view_fits_the_drawn_blocks_not_the_structure_box() {
    // a 4 x 8 x 4 structure whose top half is empty, as a tree with room above it
    let blocks: Vec<_> = (0..4u32)
        .flat_map(|x| (0..4u32).flat_map(move |y| (0..4u32).map(move |z| [x, y, z])))
        .map(|at| (at, BlockFactory::plain("stone")))
        .collect();
    let structure = PreviewFixture::structure([4, 8, 4], &blocks);
    let mesh = PreviewFixture::mesh(&structure);
    let mut rasterizer = Pictures::rasterizer();
    rasterizer.render(&mesh, OrbitCamera::start(), 160, 120);
    let [left, top, right, bottom] = Pictures::drawn_box(&rasterizer);
    // a fit on the structure box puts the blocks in the lower part of the picture and leaves the top empty
    let (middle_x, middle_y) = ((left + right) as f32 / 2.0, (top + bottom) as f32 / 2.0);
    // the perspective lifts the near top face, so the middle sits a few pixels under the centre; a box fit puts
    // it about 20 under
    assert!(
        (middle_x - 80.0).abs() <= 3.0 && (middle_y - 60.0).abs() <= 10.0,
        "drawn box {left} {top} {right} {bottom}"
    );
    // the blocks reach the fill share of the picture on one axis at least
    let reach = ((right - left) as f32 / 160.0).max((bottom - top) as f32 / 120.0);
    assert!(reach >= 0.85, "the blocks reach {reach} of the picture");
}

#[test]
fn an_empty_mesh_fits_the_structure_box_and_draws_nothing() {
    let mesh = Pictures::empty();
    let mut rasterizer = Pictures::rasterizer();
    assert_eq!(rasterizer.render(&mesh, OrbitCamera::start(), 64, 48), 0);
}

#[test]
fn a_render_with_the_stop_set_draws_nothing() {
    let stop = PreviewStop::new();
    stop.stop();
    let mut rasterizer = PreviewRasterizer::new(stop);
    assert_eq!(rasterizer.render(&Pictures::cube(), OrbitCamera::start(), 64, 48), 0);
    assert_eq!(Pictures::covered(&rasterizer), 0);
}

#[test]
fn a_swapped_out_picture_keeps_its_pixels_and_the_next_render_draws_into_the_spare() {
    let mut rasterizer = Pictures::rasterizer();
    rasterizer.render(&Pictures::cube(), OrbitCamera::start(), 64, 48);
    let drawn = rasterizer.pixels().to_vec();
    let spare = Vec::with_capacity(64 * 48 * 4);
    let spare_at = spare.as_ptr();
    let picture = rasterizer.swap_pixels(spare);
    assert_eq!(picture, drawn);
    rasterizer.render(&Pictures::cube(), OrbitCamera::start(), 64, 48);
    assert_eq!(rasterizer.pixels().as_ptr(), spare_at, "the render allocated a new buffer");
    assert_eq!(rasterizer.pixels(), &drawn[..]);
}
