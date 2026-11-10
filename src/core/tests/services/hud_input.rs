use stratum_core::services::HudInput;
use stratum_core::structs::action::HudAction;
use stratum_core::structs::hud::{HudControl, HudKey, HudMode, HudState};
use stratum_core::structs::marker::Axis;

fn full_at(control: HudControl) -> HudState {
    HudState { mode: HudMode::Full, control }
}

#[test]
fn quick_up_raises_the_layer() {
    let mut s = HudState::new();
    assert_eq!(HudInput::press(&mut s, HudKey::Up, true), Some(HudAction::LayerUp));
}

#[test]
fn quick_down_lowers_the_layer() {
    let mut s = HudState::new();
    assert_eq!(HudInput::press(&mut s, HudKey::Down, true), Some(HudAction::LayerDown));
}

#[test]
fn quick_left_toggles_shown() {
    let mut s = HudState::new();
    assert_eq!(HudInput::press(&mut s, HudKey::Left, true), Some(HudAction::ToggleShown));
}

#[test]
fn quick_right_cycles_opacity() {
    let mut s = HudState::new();
    assert_eq!(HudInput::press(&mut s, HudKey::Right, true), Some(HudAction::OpacityUp));
}

#[test]
fn mode_switches_both_ways() {
    let mut s = HudState::new();
    assert_eq!(HudInput::press(&mut s, HudKey::Mode, true), None);
    assert_eq!(s.mode, HudMode::Full);
    assert_eq!(HudInput::press(&mut s, HudKey::Mode, true), None);
    assert_eq!(s.mode, HudMode::Quick);
}

#[test]
fn full_mode_left_right_wrap_the_carousel() {
    let order = HudInput::controls(true);
    assert_eq!(
        order,
        &[
            HudControl::Layer,
            HudControl::Opacity,
            HudControl::Show,
            HudControl::Structure,
            HudControl::MoveX,
            HudControl::MoveY,
            HudControl::MoveZ,
            HudControl::Rotate,
            HudControl::EasyPlace,
        ]
    );
    let mut s = full_at(HudControl::Layer);
    for expected in order.iter().skip(1) {
        assert_eq!(HudInput::press(&mut s, HudKey::Right, true), None);
        assert_eq!(s.control, *expected);
    }
    HudInput::press(&mut s, HudKey::Right, true);
    assert_eq!(s.control, HudControl::Layer);
    HudInput::press(&mut s, HudKey::Left, true);
    assert_eq!(s.control, HudControl::EasyPlace);
}

#[test]
fn easy_place_follows_rotate_in_the_carousel() {
    for has_structures in [true, false] {
        let order = HudInput::controls(has_structures);
        let rotate = order.iter().position(|control| *control == HudControl::Rotate).unwrap();
        assert_eq!(order[rotate + 1], HudControl::EasyPlace);
        assert_eq!(order.last(), Some(&HudControl::EasyPlace));

        let mut s = full_at(HudControl::Rotate);
        HudInput::press(&mut s, HudKey::Right, has_structures);
        assert_eq!(s.control, HudControl::EasyPlace);
        HudInput::press(&mut s, HudKey::Left, has_structures);
        assert_eq!(s.control, HudControl::Rotate);
    }
}

#[test]
fn structure_is_skipped_without_structures() {
    let order = HudInput::controls(false);
    assert_eq!(order.len(), 8);
    assert!(!order.contains(&HudControl::Structure));

    let mut s = full_at(HudControl::Show);
    HudInput::press(&mut s, HudKey::Right, false);
    assert_eq!(s.control, HudControl::MoveX);
    HudInput::press(&mut s, HudKey::Left, false);
    assert_eq!(s.control, HudControl::Show);
}

#[test]
fn structure_slot_without_structures_steps_to_its_neighbours() {
    let mut s = full_at(HudControl::Structure);
    HudInput::press(&mut s, HudKey::Right, false);
    assert_eq!(s.control, HudControl::MoveX);

    let mut s = full_at(HudControl::Structure);
    HudInput::press(&mut s, HudKey::Left, false);
    assert_eq!(s.control, HudControl::Show);
}

#[test]
fn full_mode_up_down_act_on_the_current_control() {
    let cases = [
        (HudControl::Layer, HudAction::LayerUp, HudAction::LayerDown),
        (HudControl::Opacity, HudAction::OpacityUp, HudAction::OpacityDown),
        (HudControl::Show, HudAction::ToggleShown, HudAction::ToggleShown),
        (HudControl::Structure, HudAction::StructureNext, HudAction::StructurePrev),
        (HudControl::MoveX, HudAction::Move(Axis::X, 1), HudAction::Move(Axis::X, -1)),
        (HudControl::MoveY, HudAction::Move(Axis::Y, 1), HudAction::Move(Axis::Y, -1)),
        (HudControl::MoveZ, HudAction::Move(Axis::Z, 1), HudAction::Move(Axis::Z, -1)),
        (HudControl::Rotate, HudAction::Rotate(1), HudAction::Rotate(-1)),
        (HudControl::EasyPlace, HudAction::ToggleEasyPlace, HudAction::ToggleEasyPlace),
    ];
    for (control, up, down) in cases {
        let mut s = full_at(control);
        assert_eq!(HudInput::press(&mut s, HudKey::Up, true), Some(up));
        assert_eq!(HudInput::press(&mut s, HudKey::Down, true), Some(down));
        assert_eq!(s.control, control);
    }
}

#[test]
fn held_key_repeats_step_once_per_event() {
    let mut s = HudState::new();
    let actions: Vec<_> = (0..3).map(|_| HudInput::press(&mut s, HudKey::Up, true)).collect();
    assert_eq!(actions, vec![Some(HudAction::LayerUp); 3]);
}
