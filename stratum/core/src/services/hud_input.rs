use crate::structs::action::HudAction;
use crate::structs::hud::{HudControl, HudKey, HudMode, HudState};
use crate::structs::marker::Axis;

// Turns HUD key presses into marker actions and moves the Full mode carousel.
pub struct HudInput;

impl HudInput {
    const ALL: [HudControl; 9] = [
        HudControl::Layer,
        HudControl::Opacity,
        HudControl::Show,
        HudControl::Structure,
        HudControl::MoveX,
        HudControl::MoveY,
        HudControl::MoveZ,
        HudControl::Rotate,
        HudControl::EasyPlace,
    ];

    const WITHOUT_STRUCTURE: [HudControl; 8] = [
        HudControl::Layer,
        HudControl::Opacity,
        HudControl::Show,
        HudControl::MoveX,
        HudControl::MoveY,
        HudControl::MoveZ,
        HudControl::Rotate,
        HudControl::EasyPlace,
    ];

    // The carousel order. Structure is left out when no structure exists.
    pub fn controls(has_structures: bool) -> &'static [HudControl] {
        if has_structures { &Self::ALL } else { &Self::WITHOUT_STRUCTURE }
    }

    // The carousel order for the current control. A Structure control that lost its structures
    // keeps its slot, because Up and Down still act on it until Left or Right moves on.
    pub fn carousel(current: HudControl, has_structures: bool) -> &'static [HudControl] {
        Self::controls(has_structures || current == HudControl::Structure)
    }

    // One key press. Returns the action to apply, if the key makes one.
    pub fn press(state: &mut HudState, key: HudKey, has_structures: bool) -> Option<HudAction> {
        match (state.mode, key) {
            (HudMode::Quick, HudKey::Mode) => {
                state.mode = HudMode::Full;
                None
            }
            (HudMode::Full, HudKey::Mode) => {
                state.mode = HudMode::Quick;
                None
            }
            (HudMode::Quick, HudKey::Up) => Some(HudAction::LayerUp),
            (HudMode::Quick, HudKey::Down) => Some(HudAction::LayerDown),
            (HudMode::Quick, HudKey::Left) => Some(HudAction::ToggleShown),
            (HudMode::Quick, HudKey::Right) => Some(HudAction::OpacityUp),
            (HudMode::Full, HudKey::Left) => {
                state.control = Self::step(state.control, -1, has_structures);
                None
            }
            (HudMode::Full, HudKey::Right) => {
                state.control = Self::step(state.control, 1, has_structures);
                None
            }
            (HudMode::Full, HudKey::Up) => Some(Self::act(state.control, 1)),
            (HudMode::Full, HudKey::Down) => Some(Self::act(state.control, -1)),
        }
    }

    // Moves one slot, wrapping. The slot is found in the full order, so a Structure
    // control that lost its structures still steps to its neighbours (Show, MoveX).
    fn step(current: HudControl, delta: isize, has_structures: bool) -> HudControl {
        let len = Self::ALL.len() as isize;
        let mut index = Self::ALL.iter().position(|control| *control == current).unwrap_or(0) as isize;
        loop {
            index = (index + delta).rem_euclid(len);
            let next = Self::ALL[index as usize];
            if has_structures || next != HudControl::Structure {
                return next;
            }
        }
    }

    fn act(control: HudControl, sign: i8) -> HudAction {
        let up = sign > 0;
        match control {
            HudControl::Layer if up => HudAction::LayerUp,
            HudControl::Layer => HudAction::LayerDown,
            HudControl::Opacity if up => HudAction::OpacityUp,
            HudControl::Opacity => HudAction::OpacityDown,
            HudControl::Show => HudAction::ToggleShown,
            HudControl::Structure if up => HudAction::StructureNext,
            HudControl::Structure => HudAction::StructurePrev,
            HudControl::MoveX => HudAction::Move(Axis::X, sign),
            HudControl::MoveY => HudAction::Move(Axis::Y, sign),
            HudControl::MoveZ => HudAction::Move(Axis::Z, sign),
            HudControl::Rotate => HudAction::Rotate(sign),
            HudControl::EasyPlace => HudAction::ToggleEasyPlace,
        }
    }
}
