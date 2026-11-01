use stratum_core::structs::marker::TargetState;
use stratum_core::structs::snapshot::LibrarySnapshot;

use super::SidebarRow;

// Builds the sidebar rows from a snapshot.
pub struct SidebarRows;

impl SidebarRows {
    // All structures, each project by name, Unsorted, then the markers: the active one first, then by name.
    pub fn build(snapshot: &LibrarySnapshot) -> Vec<SidebarRow> {
        let mut rows = vec![SidebarRow::All { count: Self::count(snapshot.all_count) }];

        let mut projects: Vec<_> = snapshot.projects.iter().collect();
        projects.sort_by(|a, b| {
            a.name.to_lowercase().cmp(&b.name.to_lowercase()).then_with(|| a.name.cmp(&b.name)).then(a.id.cmp(&b.id))
        });
        rows.extend(projects.into_iter().map(|project| SidebarRow::Project {
            id: project.id,
            name: project.name.clone(),
            count: Self::count(project.structures.len()),
        }));
        rows.push(SidebarRow::Unsorted { count: Self::count(snapshot.unsorted.len()) });

        let mut markers: Vec<_> = snapshot.markers.iter().collect();
        markers.sort_by(|a, b| {
            b.active
                .cmp(&a.active)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                .then_with(|| a.name.cmp(&b.name))
                .then(a.id.cmp(&b.id))
        });
        rows.extend(markers.into_iter().map(|marker| SidebarRow::Marker {
            id: marker.id,
            name: marker.name.clone(),
            target: marker.target,
            target_name: marker.target_name.clone(),
            origin: marker.origin,
            active: marker.active,
            missing: marker.state == TargetState::Missing,
        }));
        rows
    }

    fn count(len: usize) -> u32 {
        u32::try_from(len).unwrap_or(u32::MAX)
    }
}
