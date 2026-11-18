use stratum_core::structs::action::{BuildAction, LibraryAction};
use stratum_core::structs::build::BuildPhase;
use stratum_core::structs::library::StructureId;
use stratum_core::structs::snapshot::LibrarySnapshot;
use stratum_ui::state::{Dialog, Page};

use crate::support::{BuildFixture, ShapeProbe, SnapshotFixture, ViewSession};

// Sessions on the page of the structure "Main hall" (id 1), as on the structure facts artboards.
struct Facts;

impl Facts {
    fn session(snapshot: LibrarySnapshot) -> ViewSession {
        let mut session = ViewSession::new(snapshot);
        session.state.page = Page::Structure(StructureId(1));
        session.frame(Vec::new());
        session
    }

    fn allowed(id: Option<u64>) -> LibrarySnapshot {
        let mut snapshot = SnapshotFixture::library();
        snapshot.build_allowed = id.map(StructureId);
        snapshot
    }

    fn running(phase: BuildPhase) -> LibrarySnapshot {
        let mut snapshot = Self::allowed(Some(1));
        snapshot.build = Some(BuildFixture::running(phase));
        snapshot
    }

    fn has(session: &ViewSession, label: &str) -> bool {
        !ShapeProbe::exact_rects(&session.shapes, label).is_empty()
    }

    fn click(session: &mut ViewSession, label: &str) {
        let spot = ShapeProbe::last_exact_centre(&session.shapes, label).expect("button is painted");
        session.click(spot);
    }
}

#[test]
fn build_shows_only_when_allowed_for_this_structure() {
    let session = Facts::session(Facts::allowed(Some(1)));
    assert!(Facts::has(&session, "Build"), "allowed for this structure");

    let session = Facts::session(Facts::allowed(None));
    assert!(!Facts::has(&session, "Build"), "no condition holds");

    let session = Facts::session(Facts::allowed(Some(3)));
    assert!(!Facts::has(&session, "Build"), "another structure is allowed");
}

#[test]
fn build_opens_the_dialog_and_sends_open() {
    let mut session = Facts::session(Facts::allowed(Some(1)));
    Facts::click(&mut session, "Build");
    assert_eq!(session.actions, [LibraryAction::Build(BuildAction::Open(StructureId(1)))]);
    assert!(matches!(session.state.dialog, Some(Dialog::Build(_))), "the settings panel is open");
}

#[test]
fn pause_resume_and_stop_show_during_a_build() {
    let mut session = Facts::session(Facts::running(BuildPhase::Building));
    assert!(!Facts::has(&session, "Build"), "Pause and Stop replace Build");
    assert!(Facts::has(&session, "Rename"), "the other buttons stay");
    Facts::click(&mut session, "Pause");
    assert_eq!(session.actions, [LibraryAction::Build(BuildAction::Pause)]);

    session.actions.clear();
    session.snapshot = Facts::running(BuildPhase::Paused);
    session.frame(Vec::new());
    assert!(!Facts::has(&session, "Pause"));
    Facts::click(&mut session, "Resume");
    assert_eq!(session.actions, [LibraryAction::Build(BuildAction::Resume)]);

    session.actions.clear();
    Facts::click(&mut session, "Stop");
    assert_eq!(session.actions, [LibraryAction::Build(BuildAction::Stop)]);
}

#[test]
fn no_build_button_while_another_build_runs() {
    let mut snapshot = Facts::running(BuildPhase::Clearing);
    snapshot.build.as_mut().expect("row").structure = StructureId(3);
    let session = Facts::session(snapshot);
    assert!(!Facts::has(&session, "Build"), "one build at a time");
    assert!(!Facts::has(&session, "Pause"), "the running build is of another structure");
}
