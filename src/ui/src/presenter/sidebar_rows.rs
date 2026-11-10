use stratum_core::structs::marker::{MarkerTarget, TargetState};
use stratum_core::structs::snapshot::{LibrarySnapshot, MarkerRow};

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
            target: Self::opens(marker),
            target_name: marker.target_name.clone(),
            origin: marker.origin,
            active: marker.active,
            missing: marker.state == TargetState::Missing,
            part: Self::part(marker),
        }));
        rows
    }

    // The page a click on the row opens: the structure on screen for a marker that has one, else its target.
    fn opens(marker: &MarkerRow) -> MarkerTarget {
        marker.current.map_or(marker.target, MarkerTarget::Structure)
    }

    fn part(marker: &MarkerRow) -> Option<(String, u32, u32)> {
        let name = marker.part_name.clone()?;
        let (number, count) = marker.part_position?;
        Some((name, number, count))
    }

    fn count(len: usize) -> u32 {
        u32::try_from(len).unwrap_or(u32::MAX)
    }
}
