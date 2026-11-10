use stratum_core::structs::library::ProjectId;
use stratum_core::structs::marker::{MarkerId, MarkerTarget};

// One row of the sidebar, in the order the sidebar draws them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SidebarRow {
    All {
        count: u32,
    },
    Project {
        id: ProjectId,
        name: String,
        count: u32,
    },
    Unsorted {
        count: u32,
    },
    Marker {
        id: MarkerId,
        name: String,
        // The page a click opens: the structure on screen for a marker that has one, else the marker's target.
        target: MarkerTarget,
        // The name of what the marker shows; empty when it is missing.
        target_name: String,
        // The marker's block, x y z.
        origin: [i32; 3],
        active: bool,
        missing: bool,
        // The structure a cycling marker shows now, its number and the count of the project's structures.
        part: Option<(String, u32, u32)>,
    },
}
