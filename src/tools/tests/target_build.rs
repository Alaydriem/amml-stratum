use std::collections::HashMap;
use std::sync::Arc;

use stratum_core::services::{GhostAssembler, StructureParser};
use stratum_core::structs::ghost::{GhostBuild, GhostJob, GhostPalette};
use stratum_core::structs::library::{Library, Project, ProjectId, ProjectKind, StructureEntry, StructureId};
use stratum_core::structs::marker::{Marker, MarkerId, MarkerTarget, MarkerView, Rotation};
use stratum_core::structs::structure::Structure;
use stratum_tools::SampleBuilder;

use crate::entry_fixture::EntryFixture;

// The eight target pieces as one big build project, assembled into a ghost build at a marker.
pub struct TargetBuild;

impl TargetBuild {
    // More than a mesh holds per cell: the target's 1.87 M cells measured 359 MB.
    pub const BYTES_PER_CELL: u64 = 300;

    pub fn at(origin: [i32; 3]) -> GhostBuild {
        let pieces: Vec<Arc<Structure>> = SampleBuilder::target_pieces()
            .unwrap()
            .iter()
            .map(|bytes| Arc::new(StructureParser::parse(bytes).unwrap()))
            .collect();
        let mut library = Library::new();
        library.projects.push(Project { id: ProjectId(1), name: "target".to_string(), kind: ProjectKind::BigBuild });
        let mut loaded = HashMap::new();
        for (index, piece) in pieces.iter().enumerate() {
            let id = index as u64 + 1;
            library.structures.push(StructureEntry { project: Some(ProjectId(1)), ..EntryFixture::of(id, piece) });
            loaded.insert(StructureId(id), Arc::clone(piece));
        }
        let marker = Marker {
            id: MarkerId(1),
            name: "Target".to_string(),
            origin,
            rotation: Rotation::R0,
            target: MarkerTarget::Project(ProjectId(1)),
            view: MarkerView::default(),
        };
        GhostAssembler::assemble(&marker, &library, &loaded, 1).unwrap()
    }

    // The GPU bytes reported for a job: every filled cell of its padded grid at `BYTES_PER_CELL`.
    pub fn bytes(job: &GhostJob) -> u64 {
        job.cells.iter().filter(|cell| **cell != GhostPalette::NONE).count() as u64 * Self::BYTES_PER_CELL
    }
}
