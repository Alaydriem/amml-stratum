use stratum_core::services::MarkerControls;
use stratum_core::structs::action::HudAction;
use stratum_core::structs::library::StructureId;
use stratum_core::structs::marker::{Axis, Marker, MarkerTarget, Rotation, ViewMode};

use crate::support::MarkerFixture;

struct Scene;

impl Scene {
    fn marker() -> Marker {
        let mut svc = MarkerFixture::service(4);
        svc.place("Home", [10, 64, -20], MarkerTarget::Structure(StructureId(1))).unwrap();
        svc.markers()[0].clone()
    }

    // A marker that shows its bottom layer.
    fn layered() -> Marker {
        let mut marker = Self::marker();
        marker.view.view_mode = ViewMode::Layer;
        marker
    }

    fn press(marker: &mut Marker, action: HudAction, layers: u32) {
        MarkerControls::apply(marker, action, layers, 1);
    }

    fn shown(marker: &Marker, layers: u32) -> Option<u32> {
        marker.view.shown_layer(layers)
    }
}

#[test]
fn layer_up_walks_bottom_to_top_then_whole_then_bottom() {
    let mut marker = Scene::layered();
    let mut seen = vec![Scene::shown(&marker, 3)];
    for _ in 0..4 {
        Scene::press(&mut marker, HudAction::LayerUp, 3);
        seen.push(Scene::shown(&marker, 3));
    }
    assert_eq!(seen, [Some(0), Some(1), Some(2), None, Some(0)]);
}

#[test]
fn layer_down_walks_bottom_to_whole_to_top() {
    let mut marker = Scene::layered();
    let mut seen = vec![Scene::shown(&marker, 3)];
    for _ in 0..4 {
        Scene::press(&mut marker, HudAction::LayerDown, 3);
        seen.push(Scene::shown(&marker, 3));
    }
    assert_eq!(seen, [Some(0), None, Some(2), Some(1), Some(0)]);
}

#[test]
fn layer_past_the_top_after_a_smaller_structure_clamps_to_whole() {
    let mut marker = Scene::marker();
    marker.view.layer = 7;
    assert_eq!(Scene::shown(&marker, 3), None);

    Scene::press(&mut marker, HudAction::LayerUp, 3);
    assert_eq!(Scene::shown(&marker, 3), Some(0));

    marker.view.layer = 7;
    marker.view.view_mode = ViewMode::Layer;
    Scene::press(&mut marker, HudAction::LayerDown, 3);
    assert_eq!(Scene::shown(&marker, 3), Some(2));
}

#[test]
fn layer_actions_do_nothing_without_layers() {
    let mut marker = Scene::marker();
    let before = marker.view.clone();
    Scene::press(&mut marker, HudAction::LayerUp, 0);
    Scene::press(&mut marker, HudAction::LayerDown, 0);
    assert_eq!(marker.view, before);
}

#[test]
fn opacity_up_wraps_seven_to_zero() {
    let mut marker = Scene::marker();
    marker.view.opacity_step = 6;
    Scene::press(&mut marker, HudAction::OpacityUp, 1);
    assert_eq!(marker.view.opacity_step, 7);
    assert_eq!(marker.view.opacity().alpha(), 1.0);
    Scene::press(&mut marker, HudAction::OpacityUp, 1);
    assert_eq!(marker.view.opacity_step, 0);
}

#[test]
fn opacity_down_wraps_zero_to_seven() {
    let mut marker = Scene::marker();
    marker.view.opacity_step = 1;
    Scene::press(&mut marker, HudAction::OpacityDown, 1);
    assert_eq!(marker.view.opacity_step, 0);
    assert_eq!(marker.view.opacity().alpha(), 0.30);
    Scene::press(&mut marker, HudAction::OpacityDown, 1);
    assert_eq!(marker.view.opacity_step, 7);
}

#[test]
fn new_markers_start_at_half_opacity() {
    assert_eq!(Scene::marker().view.opacity().alpha(), 0.50);
}

#[test]
fn move_changes_origin_by_one_block_per_axis() {
    let cases = [
        (Axis::X, 1, [11, 64, -20]),
        (Axis::X, -1, [9, 64, -20]),
        (Axis::Y, 1, [10, 65, -20]),
        (Axis::Y, -1, [10, 63, -20]),
        (Axis::Z, 1, [10, 64, -19]),
        (Axis::Z, -1, [10, 64, -21]),
    ];
    for (axis, direction, expected) in cases {
        let mut marker = Scene::marker();
        Scene::press(&mut marker, HudAction::Move(axis, direction), 1);
        assert_eq!(marker.origin, expected);
    }
}

#[test]
fn rotate_cycles_both_ways() {
    let mut marker = Scene::marker();
    let mut forward = Vec::new();
    for _ in 0..4 {
        Scene::press(&mut marker, HudAction::Rotate(1), 1);
        forward.push(marker.rotation);
    }
    assert_eq!(forward, [Rotation::R90, Rotation::R180, Rotation::R270, Rotation::R0]);

    let mut back = Vec::new();
    for _ in 0..4 {
        Scene::press(&mut marker, HudAction::Rotate(-1), 1);
        back.push(marker.rotation);
    }
    assert_eq!(back, [Rotation::R270, Rotation::R180, Rotation::R90, Rotation::R0]);
}

#[test]
fn toggle_shown_keeps_every_other_setting() {
    let mut marker = Scene::marker();
    marker.view.opacity_step = 5;
    marker.view.layer = 2;
    marker.view.selected = 1;
    let mut expected = marker.clone();

    Scene::press(&mut marker, HudAction::ToggleShown, 3);
    expected.view.shown = false;
    assert_eq!(marker, expected);

    Scene::press(&mut marker, HudAction::ToggleShown, 3);
    expected.view.shown = true;
    assert_eq!(marker, expected);
}

#[test]
fn toggle_easy_place_flips_only_that_switch() {
    let mut marker = Scene::marker();
    marker.view.opacity_step = 5;
    let mut expected = marker.clone();
    assert!(!marker.view.easy_place);

    Scene::press(&mut marker, HudAction::ToggleEasyPlace, 3);
    expected.view.easy_place = true;
    assert_eq!(marker, expected);

    Scene::press(&mut marker, HudAction::ToggleEasyPlace, 3);
    expected.view.easy_place = false;
    assert_eq!(marker, expected);
}

#[test]
fn structure_next_wraps_within_the_project() {
    let mut marker = Scene::marker();
    let mut seen = Vec::new();
    for _ in 0..4 {
        MarkerControls::apply(&mut marker, HudAction::StructureNext, 1, 3);
        seen.push(marker.view.selected);
    }
    assert_eq!(seen, [1, 2, 0, 1]);
    for _ in 0..2 {
        MarkerControls::apply(&mut marker, HudAction::StructurePrev, 1, 3);
    }
    assert_eq!(marker.view.selected, 2);
}

#[test]
fn structure_actions_stay_on_the_first_structure_when_there_is_one() {
    let mut marker = Scene::marker();
    MarkerControls::apply(&mut marker, HudAction::StructureNext, 1, 1);
    MarkerControls::apply(&mut marker, HudAction::StructurePrev, 1, 1);
    assert_eq!(marker.view.selected, 0);
}
