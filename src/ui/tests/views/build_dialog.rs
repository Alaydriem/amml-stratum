use egui::Key;
use stratum_core::structs::action::{BuildAction, LibraryAction};
use stratum_core::structs::build::{BuildNotice, BuildPhase, BuildSettings};
use stratum_core::structs::library::StructureId;
use stratum_core::structs::snapshot::BuildRow;
use stratum_ui::state::{Dialog, Page};
use stratum_ui::views::dialogs::BuildDialog;

use crate::support::{BuildFixture, ShapeProbe, SnapshotFixture, ViewSession};

// A session on the page of "Main hall" (id 1) with the build dialog open over it, as on the Build dialog artboards.
struct Panel;

impl Panel {
    fn open(row: Option<BuildRow>) -> ViewSession {
        let mut snapshot = SnapshotFixture::library();
        snapshot.build_allowed = Some(StructureId(1));
        snapshot.build = row;
        let mut session = ViewSession::new(snapshot);
        session.state.page = Page::Structure(StructureId(1));
        session.state.dialog = Some(Dialog::Build(BuildDialog::new(StructureId(1))));
        // a new modal paints from its second frame
        session.frame(Vec::new());
        session.frame(Vec::new());
        session
    }

    fn has(session: &ViewSession, text: &str) -> bool {
        !ShapeProbe::exact_rects(&session.shapes, text).is_empty()
    }

    fn click(session: &mut ViewSession, text: &str) {
        let spot = ShapeProbe::last_text_centre(&session.shapes, text).expect("painted");
        session.click(spot);
    }

    // Types `rate` into the rate field, which shows `shown` now.
    fn set_rate(session: &mut ViewSession, shown: &str, rate: &str) {
        let spot = ShapeProbe::last_exact_centre(&session.shapes, shown).expect("the rate is painted");
        session.click(spot);
        session.type_text(rate);
        session.press(Key::Enter);
    }

    fn set(rate: u8, clear_water: bool, clear_lava: bool) -> LibraryAction {
        let settings = BuildSettings { blocks_per_second: rate, clear_water, clear_lava };
        LibraryAction::Build(BuildAction::SetSettings(StructureId(1), settings))
    }
}

#[test]
fn the_dialog_sends_settings_and_start() {
    let mut session = Panel::open(Some(BuildFixture::ready()));
    Panel::set_rate(&mut session, "4", "8");
    Panel::click(&mut session, "Clear water");
    Panel::click(&mut session, "Start");
    assert_eq!(
        session.actions,
        [Panel::set(8, true, true), Panel::set(8, false, true), LibraryAction::Build(BuildAction::Start)]
    );
}

#[test]
fn the_estimate_follows_the_rate() {
    let mut session = Panel::open(Some(BuildFixture::ready()));
    assert!(Panel::has(&session, "17 min 33 s"), "4,212 blocks at 4 a second");
    Panel::set_rate(&mut session, "4", "8");
    assert!(Panel::has(&session, "8 min 47 s"), "4,212 blocks at 8 a second, rounded up");
    assert!(!Panel::has(&session, "17 min 33 s"));
}

#[test]
fn start_is_disabled_with_no_block_to_place() {
    let row = BuildRow { total: 0, eta_seconds: 0, ..BuildFixture::ready() };
    let mut session = Panel::open(Some(row));
    assert!(Panel::has(&session, "0 s"));
    Panel::click(&mut session, "Start");
    assert!(session.actions.is_empty(), "{:?}", session.actions);
    assert!(session.state.dialog.is_some());
}

#[test]
fn start_waits_while_the_blocks_are_read() {
    let row = BuildRow { phase: BuildPhase::Probing, total: 0, ..BuildFixture::ready() };
    let mut session = Panel::open(Some(row));
    Panel::click(&mut session, "Start");
    assert!(session.actions.is_empty(), "{:?}", session.actions);
    assert!(session.state.dialog.is_some());

    session.snapshot.build = Some(BuildFixture::ready());
    session.frame(Vec::new());
    Panel::click(&mut session, "Start");
    assert_eq!(session.actions, [LibraryAction::Build(BuildAction::Start)]);
}

#[test]
fn the_dialog_lists_unplaceable_blocks() {
    let row =
        BuildRow { unplaceable: vec!["Structure void".to_owned(), "Jigsaw block".to_owned()], ..BuildFixture::ready() };
    let session = Panel::open(Some(row));
    assert!(Panel::has(&session, "BLOCKS THAT CANNOT BE PLACED"));
    assert!(Panel::has(&session, "Structure void"));
    assert!(Panel::has(&session, "Jigsaw block"));

    let session = Panel::open(Some(BuildFixture::ready()));
    assert!(!Panel::has(&session, "BLOCKS THAT CANNOT BE PLACED"), "no list when every block can be placed");
}

#[test]
fn a_notice_shows_only_when_the_row_has_one() {
    let session = Panel::open(Some(BuildFixture::ready()));
    assert!(!Panel::has(&session, "Stand near the structure to keep water or lava"));
    assert!(!Panel::has(&session, "Too many block kinds to keep water or lava"));

    let near = BuildRow { notice: Some(BuildNotice::StandNear), ..BuildFixture::ready() };
    let session = Panel::open(Some(near));
    assert!(Panel::has(&session, "Stand near the structure to keep water or lava"));

    let many = BuildRow { notice: Some(BuildNotice::TooManyKinds), ..BuildFixture::ready() };
    let session = Panel::open(Some(many));
    assert!(Panel::has(&session, "Too many block kinds to keep water or lava"));
}

#[test]
fn the_dialog_names_the_structure_and_its_size() {
    let session = Panel::open(Some(BuildFixture::ready()));
    assert!(Panel::has(&session, "Main hall · 32 × 18 × 24 · 4,212 blocks"));
    assert!(Panel::has(&session, "Build structure"));
}

#[test]
fn cancel_sends_cancel_and_closes() {
    let mut session = Panel::open(Some(BuildFixture::ready()));
    Panel::click(&mut session, "Cancel");
    assert_eq!(session.actions, [LibraryAction::Build(BuildAction::Cancel)]);
    assert!(session.state.dialog.is_none());
}

#[test]
fn the_dialog_closes_when_the_build_runs() {
    let mut session = Panel::open(Some(BuildFixture::ready()));
    session.snapshot.build = Some(BuildFixture::running(BuildPhase::Clearing));
    session.frame(Vec::new());
    assert!(session.state.dialog.is_none());
    assert!(session.actions.is_empty(), "{:?}", session.actions);
}

#[test]
fn the_dialog_closes_when_building_is_no_longer_allowed() {
    let mut session = Panel::open(None);
    assert!(session.state.dialog.is_some(), "the row is still on its way");
    session.snapshot.build_allowed = None;
    session.frame(Vec::new());
    assert!(session.state.dialog.is_none());
}
