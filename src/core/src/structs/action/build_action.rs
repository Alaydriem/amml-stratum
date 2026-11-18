use crate::structs::build::BuildSettings;
use crate::structs::library::StructureId;

// A change the UI or the HUD asks of the build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildAction {
    // Opens the settings panel of a structure. The structure must be the one `build_allowed` names.
    Open(StructureId),
    // Saves the settings of a structure. They apply to the open panel until Start.
    SetSettings(StructureId, BuildSettings),
    Start,
    Pause,
    Resume,
    Stop,
    // Closes the panel or the card of a build that is not running.
    Cancel,
}
