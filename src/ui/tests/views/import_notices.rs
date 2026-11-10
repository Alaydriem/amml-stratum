use stratum_core::structs::import::ImportFailure;
use stratum_core::structs::marker::TargetState;
use stratum_core::structs::snapshot::ImportNotice;

use crate::support::{ShapeProbe, SnapshotFixture, ViewSession};

#[test]
fn every_notice_is_shown() {
    let mut snapshot = SnapshotFixture::library();
    snapshot.notices = vec![
        ImportNotice::ImportFailed { file: "castle.mcstructure".to_owned(), reason: ImportFailure::Unreadable },
        ImportNotice::ImportQueueFull,
        ImportNotice::MaterialsBusy,
    ];
    let session = ViewSession::new(snapshot);
    assert!(ShapeProbe::text_rect(&session.shapes, "castle.mcstructure").is_some());
    assert!(ShapeProbe::text_rect(&session.shapes, "waiting").is_some(), "the queue full notice is shown");
    assert!(ShapeProbe::text_rect(&session.shapes, "busy").is_some(), "the materials busy notice is shown");
}

#[test]
fn only_the_newest_notices_are_shown_newest_first() {
    let mut snapshot = SnapshotFixture::library();
    snapshot.notices = (1..=5)
        .map(|number| ImportNotice::ImportFailed {
            file: format!("file{number}.mcstructure"),
            reason: ImportFailure::Unreadable,
        })
        .collect();
    let session = ViewSession::new(snapshot);
    for gone in ["file1.mcstructure", "file2.mcstructure"] {
        assert!(ShapeProbe::text_rect(&session.shapes, gone).is_none(), "{gone} is not shown");
    }
    let newest = ShapeProbe::text_rect(&session.shapes, "file5.mcstructure").expect("the newest notice is shown");
    let middle = ShapeProbe::text_rect(&session.shapes, "file4.mcstructure").expect("the second notice is shown");
    let oldest = ShapeProbe::text_rect(&session.shapes, "file3.mcstructure").expect("the third notice is shown");
    assert!(newest.min.y < middle.min.y && middle.min.y < oldest.min.y);
}

#[test]
fn many_missing_markers_share_one_banner() {
    let mut snapshot = SnapshotFixture::library();
    snapshot.markers = (1..=3)
        .map(|number| {
            SnapshotFixture::marker(number, &format!("Lost {number}"), [10, 64, -340], false, TargetState::Missing)
        })
        .collect();
    let session = ViewSession::new(snapshot);
    assert!(ShapeProbe::text_rect(&session.shapes, "3 markers have nothing to show").is_some());
    assert!(ShapeProbe::text_rect(&session.shapes, "has nothing to show").is_none());
}

#[test]
fn one_missing_marker_keeps_its_own_banner() {
    let session = ViewSession::new(SnapshotFixture::library());
    assert!(ShapeProbe::text_rect(&session.shapes, "Lighthouse has nothing to show").is_some());
}
