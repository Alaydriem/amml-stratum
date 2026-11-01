use crate::structs::marker::Axis;

// A change the player makes to the active marker with a key, without the library open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudAction {
    LayerUp,
    LayerDown,
    ToggleShown,
    OpacityUp,
    OpacityDown,
    // One block along a world axis per press. Only the sign of the step counts; its size is ignored.
    Move(Axis, i8),
    // Quarter turns around the vertical axis; the sign is the direction.
    Rotate(i8),
    StructureNext,
    StructurePrev,
    ToggleEasyPlace,
}
