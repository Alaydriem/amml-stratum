use stratum_core::services::GhostSections;
use stratum_core::structs::ghost::{GhostBuild, GhostJob, GhostPalette, GhostView, SectionKey, SectionPart};

// Builds keys and reads padded job grids for section tests.
pub struct SectionAssert;

impl SectionAssert {
    pub fn key(x: i32, y: i32, z: i32, layer: Option<i32>, part: SectionPart) -> SectionKey {
        SectionKey { x, y, z, layer, part }
    }

    pub fn whole(x: i32, y: i32, z: i32) -> SectionKey {
        Self::key(x, y, z, None, SectionPart::Whole)
    }

    // Index into a padded grid, X fastest, then Z, then Y.
    pub fn at(dims: [usize; 3], x: usize, y: usize, z: usize) -> usize {
        (y * (dims[2] + 2) + z) * (dims[0] + 2) + x
    }

    pub fn filled(job: &GhostJob) -> usize {
        job.cells.iter().filter(|cell| **cell != GhostPalette::NONE).count()
    }

    // The wanted keys for a build, scanning its occupied sections first.
    pub fn wanted(build: &GhostBuild, view: GhostView, feet: [i32; 3], draw_distance: u32) -> Vec<SectionKey> {
        let mut occupied = Vec::new();
        GhostSections::occupied(build, 600, &mut occupied, &mut Vec::new());
        let mut out = Vec::new();
        GhostSections::wanted(&occupied, view, feet, draw_distance, &mut out);
        out
    }
}
