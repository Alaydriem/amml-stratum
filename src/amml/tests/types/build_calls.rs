use alaydriem_stratum::StratumMod;
use alaydriem_stratum::errors::GameLayoutError;
use alaydriem_stratum::types::{BuildCalls, GhostCalls, StratumEntry};

use crate::support::{FakeGhost, FakeResolver};

fn calls_of(values: &[(amml_game::EntryId, u64)]) -> Result<BuildCalls, GameLayoutError> {
    let entries = StratumMod::resolve(&FakeResolver::new(values)).unwrap();
    let ghost = GhostCalls::new(&entries, FakeGhost::heap_new, FakeGhost::heap_delete).unwrap();
    BuildCalls::new(&entries, &ghost)
}

#[test]
fn with_every_entry_the_build_calls_are_made() {
    assert!(calls_of(&FakeResolver::values()).is_ok());
}

#[test]
fn a_missing_build_entry_names_the_entry() {
    for missing in [
        StratumEntry::PLAYER_GET_PLAYER_PERMISSION_LEVEL,
        StratumEntry::LEVEL_DATA_COMMANDS_ENABLED,
        StratumEntry::COMMAND_REQUEST_PACKET_M_COMMAND,
        StratumEntry::COMMAND_REQUEST_PACKET_SIZE,
    ] {
        let values: Vec<_> = FakeResolver::values().into_iter().filter(|(id, _)| *id != missing).collect();
        let entries = StratumMod::resolve(&FakeResolver::new(&values)).unwrap();
        let ghost = GhostCalls::new(&entries, FakeGhost::heap_new, FakeGhost::heap_delete).unwrap();

        let result = BuildCalls::new(&entries, &ghost).map(|_| ());

        assert_eq!(result, Err(GameLayoutError::Missing { entry: missing.as_str() }));
    }
}

#[test]
fn a_command_string_that_does_not_fit_the_packet_is_refused() {
    let values = FakeResolver::with(StratumEntry::COMMAND_REQUEST_PACKET_SIZE, 0x40);

    let result = calls_of(&values).map(|_| ());

    assert!(
        matches!(result, Err(GameLayoutError::Misfit { entry: "CommandRequestPacket.mCommand", .. })),
        "{result:?}"
    );
}
