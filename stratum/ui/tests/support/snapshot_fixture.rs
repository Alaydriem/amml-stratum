use stratum_core::structs::action::MaterialScope;
use stratum_core::structs::library::{ProjectId, ProjectKind, StructureId};
use stratum_core::structs::marker::{MarkerId, MarkerTarget, TargetState};
use stratum_core::structs::material::{ItemId, MaterialLine, MaterialList};
use stratum_core::structs::snapshot::{LibrarySnapshot, MarkerRow, ProjectRow, StructureRow};
use stratum_core::structs::structure::StructureSize;

// Snapshots the view and presenter tests share.
pub struct SnapshotFixture;

impl SnapshotFixture {
    pub fn structure(id: u64, name: &str, project: Option<u64>) -> StructureRow {
        StructureRow {
            id: StructureId(id),
            name: name.to_owned(),
            size: StructureSize::new(32, 18, 24),
            block_count: 4212,
            project: project.map(ProjectId),
            grid_cell: None,
            origin: None,
            file_name: String::new(),
        }
    }

    pub fn project(id: u64, name: &str, structures: Vec<StructureRow>) -> ProjectRow {
        ProjectRow { id: ProjectId(id), name: name.to_owned(), kind: ProjectKind::Normal, structures }
    }

    pub fn marker(id: u64, name: &str, origin: [i32; 3], active: bool, state: TargetState) -> MarkerRow {
        MarkerRow {
            id: MarkerId(id),
            name: name.to_owned(),
            target: MarkerTarget::Structure(StructureId(1)),
            target_name: if state == TargetState::Ok { "Main hall".to_owned() } else { String::new() },
            state,
            origin,
            active,
        }
    }

    // Two projects, two unsorted structures and two markers, as on the Library artboard.
    pub fn library() -> LibrarySnapshot {
        let mut snapshot = LibrarySnapshot::empty();
        snapshot.revision = 1;
        snapshot.projects = vec![
            Self::project(2, "Willow Manor", vec![Self::structure(1, "Main hall", Some(2))]),
            Self::project(1, "Harbour", vec![Self::structure(3, "Harbour west", Some(1))]),
        ];
        snapshot.unsorted = vec![Self::structure(4, "dock_crane", None), Self::structure(5, "mill_wheel", None)];
        snapshot.all_count = 4;
        snapshot.markers = vec![
            Self::marker(2, "Lighthouse", [214, 70, -9], false, TargetState::Missing),
            Self::marker(1, "East wing", [112, 64, -340], true, TargetState::Ok),
        ];
        snapshot.in_world = true;
        snapshot.has_player = true;
        snapshot
    }

    pub fn line(item: &str, count: u64, stack_size: u32) -> MaterialLine {
        MaterialLine { item: ItemId(item.to_owned()), count, stack_size }
    }

    // A list like the Project materials artboard: stone bricks, spruce planks, a lantern.
    pub fn list() -> MaterialList {
        MaterialList::new(vec![
            Self::line("minecraft:lantern", 42, 64),
            Self::line("minecraft:stone_bricks", 3412, 64),
            Self::line("minecraft:spruce_planks", 2208, 64),
            Self::line("minecraft:deepslate_tiles", 1536, 64),
        ])
    }

    // The library with the material list of Willow Manor (project 2) arrived.
    pub fn library_with_materials() -> LibrarySnapshot {
        let mut snapshot = Self::library();
        snapshot.revision = 2;
        snapshot.materials = Some((MaterialScope::Project(ProjectId(2)), Self::list()));
        snapshot
    }
}
