use stratum_core::structs::marker::{MarkerId, MarkerTarget};

// The entry the user picked in an assign dialog, kept by id so a change of the library cannot move it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignChoice {
    Marker(MarkerId),
    Target(MarkerTarget),
}
