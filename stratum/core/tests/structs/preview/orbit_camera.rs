use stratum_core::structs::preview::OrbitCamera;

#[test]
fn pitch_stops_at_its_limits_and_yaw_wraps() {
    let camera = OrbitCamera::start().turned(-90.0, 500.0);
    assert_eq!(camera.pitch, OrbitCamera::MAX_PITCH);
    assert_eq!(camera.yaw, 315.0);
    let camera = camera.turned(400.0, -500.0);
    assert_eq!(camera.pitch, OrbitCamera::MIN_PITCH);
    assert_eq!(camera.yaw, 355.0);
}

#[test]
fn zoom_stops_at_its_limits_and_a_turn_keeps_it() {
    let camera = OrbitCamera::start().zoomed(2);
    assert!((camera.zoom - OrbitCamera::ZOOM_STEP * OrbitCamera::ZOOM_STEP).abs() < 1e-5);
    assert_eq!(camera.turned(10.0, 5.0).zoom, camera.zoom);
    assert_eq!((camera.zoomed(-1).yaw, camera.zoomed(-1).pitch), (camera.yaw, camera.pitch));
    assert_eq!(camera.zoomed(100).zoom, OrbitCamera::MAX_ZOOM);
    assert_eq!(camera.zoomed(-100).zoom, OrbitCamera::MIN_ZOOM);
}
