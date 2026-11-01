use stratum_core::structs::marker::{MarkerId, MarkerTarget};

// What an assign dialog starts from: a structure or project that waits for a marker, or a marker that waits for a
// structure or project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignSubject {
    Target(MarkerTarget),
    Marker(MarkerId),
}
