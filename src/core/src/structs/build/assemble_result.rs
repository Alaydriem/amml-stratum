use crate::errors::GhostError;
use crate::structs::ghost::GhostBuild;

// The worker's answer to an `AssembleRequest`.
#[derive(Debug)]
pub struct AssembleResult {
    pub serial: u64,
    pub build: Result<GhostBuild, GhostError>,
}
