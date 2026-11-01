use amm_client::test_support::FakeTable;
use amm_core::structs::descriptor::Capability;
use stratum_core::structs::easy_place::{SpotWorld, UseCommand, UseProbe};

use crate::support::{CoreSession, EasyPlace};

fn aim(session: &CoreSession) -> bool {
    EasyPlace::aim(session, &EasyPlace::above(10, -20)).is_some()
}

fn decide(session: &CoreSession) -> UseCommand {
    let probe = EasyPlace::above(10, -20);
    let slots = EasyPlace::stone_in_hand();
    let world = SpotWorld { against: [true; 6] };
    session.handle().with_state(|state| state.use_decide(&probe, Some(&world), &slots, 0)).unwrap()
}

#[test]
fn the_use_path_is_vanilla_while_the_library_is_open() {
    let session = EasyPlace::start();
    let probe = EasyPlace::above(10, -20);
    EasyPlace::aim_when_ready(&session, &probe);

    session.open();

    assert!(!aim(&session), "no spot while the library is open");
    assert_eq!(decide(&session), UseCommand::Vanilla);

    FakeTable::push_event(CoreSession::OPEN_KEY);
    session.tick(&CoreSession::in_world());

    EasyPlace::aim_when_ready(&session, &probe);
    assert!(matches!(decide(&session), UseCommand::Place(_)));
}

#[test]
fn the_use_path_is_vanilla_while_another_mod_holds_input() {
    let session = EasyPlace::start();
    EasyPlace::aim_when_ready(&session, &EasyPlace::above(10, -20));

    FakeTable::set_held_by_other(Capability::Input, true);
    session.tick(&CoreSession::in_world());

    assert!(!aim(&session));
    assert_eq!(decide(&session), UseCommand::Vanilla);

    FakeTable::set_held_by_other(Capability::Input, false);
    session.tick(&CoreSession::in_world());

    assert!(aim(&session));
}

// Looks away from the build, then asks what to do with the use.
fn away_command(session: &CoreSession) -> UseCommand {
    let probe = UseProbe { dir: [0.0, 1.0, 0.0], ..EasyPlace::above(10, -20) };
    let slots = EasyPlace::stone_in_hand();
    assert_eq!(EasyPlace::aim(session, &probe), None);
    session.handle().with_state(|state| state.use_decide(&probe, None, &slots, 0)).unwrap()
}

#[test]
fn closing_the_gate_ends_the_hold() {
    let session = EasyPlace::start();
    EasyPlace::aim_when_ready(&session, &EasyPlace::above(10, -20));
    assert!(matches!(decide(&session), UseCommand::Place(_)));
    assert_eq!(away_command(&session), UseCommand::Block, "a hold that placed blocks the rest of its uses");

    session.tick(&CoreSession::in_menu());
    session.tick(&CoreSession::in_world());

    assert_eq!(away_command(&session), UseCommand::Vanilla, "the hold ended when the gate closed");
}
