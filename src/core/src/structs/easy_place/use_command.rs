use super::Placement;

// What the shim does with the use call.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UseCommand {
    // Let the game handle the use.
    Vanilla,
    // Cancel the game's use and do nothing else.
    Block,
    // Change the selected slot, then place in the same call.
    Select { slot: u8, place: Placement },
    // Exchange the two stacks and place nothing. The answer comes back in `UseProbe::swap_answer`.
    Swap { from: u8, to: u8 },
    // Creative pick of the build palette block into `slot`, replacing what is there. Places nothing.
    // The answer comes back in `UseProbe::swap_answer`.
    Pick { palette_id: u16, slot: u8 },
    Place(Placement),
}
