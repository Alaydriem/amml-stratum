use crate::structs::layout::BigBuildLayout;

// What the active marker shows, resolved once for both the HUD key actions and the HUD card. It
// borrows the library and the solved layouts, so a key press allocates nothing.
pub(crate) struct HudTarget<'a> {
    // The structure shown, or the project name of a big build. Empty when the target is gone.
    pub name: &'a str,
    // The layer count of the build. Zero when the target is gone.
    pub layers: u32,
    // The structures a normal project cycles through; 1 for every other target.
    pub structures: u32,
    // The selected structure of a normal project, 1-based, and the structure count.
    pub position: Option<(u32, u32)>,
    // The solved layout of a big build, when it solved.
    pub layout: Option<&'a BigBuildLayout>,
}
