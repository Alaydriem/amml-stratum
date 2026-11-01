use stratum_core::structs::library::ProjectId;
use stratum_core::structs::marker::{MarkerId, TargetState};
use stratum_core::structs::snapshot::LibrarySnapshot;
use stratum_ui::presenter::{SidebarRow, SidebarRows};

use crate::support::SnapshotFixture;

#[test]
fn sidebar_lists_all_projects_unsorted_then_markers() {
    let rows = SidebarRows::build(&SnapshotFixture::library());
    let kinds: Vec<&str> = rows
        .iter()
        .map(|row| match row {
            SidebarRow::All { .. } => "all",
            SidebarRow::Project { .. } => "project",
            SidebarRow::Unsorted { .. } => "unsorted",
            SidebarRow::Marker { .. } => "marker",
        })
        .collect();
    assert_eq!(kinds, ["all", "project", "project", "unsorted", "marker", "marker"]);
    assert_eq!(rows[0], SidebarRow::All { count: 4 });
    // projects are sorted by name, not by id
    assert_eq!(rows[1], SidebarRow::Project { id: ProjectId(1), name: "Harbour".to_owned(), count: 1 });
    assert_eq!(rows[2], SidebarRow::Project { id: ProjectId(2), name: "Willow Manor".to_owned(), count: 1 });
    assert_eq!(rows[3], SidebarRow::Unsorted { count: 2 });
}

#[test]
fn empty_library_still_has_all_and_unsorted() {
    let rows = SidebarRows::build(&LibrarySnapshot::empty());
    assert_eq!(rows, vec![SidebarRow::All { count: 0 }, SidebarRow::Unsorted { count: 0 }]);
}

#[test]
fn markers_sort_active_first_then_by_name_ignoring_case() {
    let mut snapshot = LibrarySnapshot::empty();
    snapshot.markers = vec![
        SnapshotFixture::marker(1, "mill", [5, 64, -340], false, TargetState::Ok),
        SnapshotFixture::marker(2, "Gate", [300, 64, -340], false, TargetState::Ok),
        SnapshotFixture::marker(3, "Zenith", [900, 64, -340], true, TargetState::Ok),
        SnapshotFixture::marker(4, "Barn", [40, 64, -340], false, TargetState::Ok),
    ];
    let ids: Vec<MarkerId> = SidebarRows::build(&snapshot)
        .into_iter()
        .filter_map(|row| match row {
            SidebarRow::Marker { id, .. } => Some(id),
            _ => None,
        })
        .collect();
    assert_eq!(ids, [MarkerId(3), MarkerId(4), MarkerId(2), MarkerId(1)]);
}

#[test]
fn missing_target_marker_row_is_flagged() {
    let mut snapshot = LibrarySnapshot::empty();
    snapshot.markers = vec![
        SnapshotFixture::marker(1, "Lighthouse", [214, 64, -340], false, TargetState::Missing),
        SnapshotFixture::marker(2, "East wing", [12, 64, -340], false, TargetState::Ok),
    ];
    let flags: Vec<(String, bool)> = SidebarRows::build(&snapshot)
        .into_iter()
        .filter_map(|row| match row {
            SidebarRow::Marker { name, missing, .. } => Some((name, missing)),
            _ => None,
        })
        .collect();
    assert_eq!(flags, [("East wing".to_owned(), false), ("Lighthouse".to_owned(), true)]);
}
