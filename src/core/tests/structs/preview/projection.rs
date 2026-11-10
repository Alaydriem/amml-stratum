use stratum_core::structs::preview::{OrbitCamera, PreviewBounds, PreviewProjection};

// Boxes and corners for the projection tests.
struct Views;

impl Views {
    // A box of `size` blocks from the origin.
    fn bounds(size: [i16; 3]) -> PreviewBounds {
        let mut bounds = PreviewBounds::empty();
        bounds.include([0, 0, 0]);
        bounds.include(size.map(|side| side * 16));
        bounds
    }

    fn corners(bounds: PreviewBounds) -> Vec<[f32; 3]> {
        let (low, high) = (bounds.low(), bounds.high());
        (0..8u32)
            .map(|corner| {
                [
                    if corner & 1 == 0 { low[0] } else { high[0] },
                    if corner & 2 == 0 { low[1] } else { high[1] },
                    if corner & 4 == 0 { low[2] } else { high[2] },
                ]
            })
            .collect()
    }
}

#[test]
fn the_centre_of_the_box_lands_on_the_centre_of_the_image() {
    let bounds = Views::bounds([4, 8, 2]);
    for camera in [OrbitCamera::start(), OrbitCamera::new(200.0, -30.0), OrbitCamera::start().zoomed(4)] {
        let projection = PreviewProjection::new(bounds, camera, 160, 120);
        let [x, y, inverse] = projection.project([2.0, 4.0, 1.0]);
        assert!(inverse > 0.0);
        assert!((x - 80.0).abs() < 0.01 && (y - 60.0).abs() < 0.01, "centre at {x} {y}");
    }
}

#[test]
fn the_start_view_keeps_every_corner_inside_the_image_and_fills_it() {
    let bounds = Views::bounds([10, 3, 6]);
    let projection = PreviewProjection::new(bounds, OrbitCamera::start(), 200, 100);
    let mut reach: f32 = 0.0;
    for corner in Views::corners(bounds) {
        let [x, y, _] = projection.project(corner);
        assert!((0.0..=200.0).contains(&x) && (0.0..=100.0).contains(&y), "corner at {x} {y}");
        reach = reach.max(((x - 100.0) / 100.0).abs()).max(((y - 50.0) / 50.0).abs());
    }
    assert!((reach - PreviewProjection::FILL).abs() < 0.01, "the box reaches {reach} of the half image");
}

#[test]
fn zoom_scales_the_distance_from_the_centre() {
    let bounds = Views::bounds([2, 2, 2]);
    let camera = OrbitCamera::start().zoomed(5);
    let fitted = PreviewProjection::new(bounds, OrbitCamera::start(), 100, 100).project([2.0, 2.0, 2.0]);
    let zoomed = PreviewProjection::new(bounds, camera, 100, 100).project([2.0, 2.0, 2.0]);
    assert!(((zoomed[0] - 50.0) - (fitted[0] - 50.0) * camera.zoom).abs() < 0.01);
    assert!(((zoomed[1] - 50.0) - (fitted[1] - 50.0) * camera.zoom).abs() < 0.01);
}

#[test]
fn a_point_behind_the_eye_is_not_drawn() {
    let bounds = Views::bounds([1, 1, 1]);
    let projection = PreviewProjection::new(bounds, OrbitCamera::new(0.0, 0.0), 64, 64);
    // yaw 0 looks from +Z towards -Z, so a point far out on +Z sits behind the eye
    let [_, _, inverse] = projection.project([0.5, 0.5, 500.0]);
    assert!(inverse <= 0.0);
}
