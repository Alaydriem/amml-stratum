use alaydriem_stratum::StratumMod;
use alaydriem_stratum::types::{BlockActorCalls, StratumEntry, StratumFeature};
use amml_game::EntryId;

use crate::support::{FakeBlockActor, FakeResolver};

// the off lines and the calls with every entry but `missing`
fn without(missing: EntryId) -> (Vec<String>, Option<BlockActorCalls>) {
    let values: Vec<_> = FakeResolver::values().into_iter().filter(|(id, _)| *id != missing).collect();
    let entries = StratumMod::resolve(&FakeResolver::new(&values)).expect("the block-entity features are optional");
    (StratumMod::off_lines(&entries), BlockActorCalls::new(&entries, FakeBlockActor::heap_delete))
}

#[test]
fn a_missing_block_actor_entry_turns_the_feature_off() {
    let every = StratumMod::resolve(&FakeResolver::new(&FakeResolver::values())).unwrap();
    let calls = BlockActorCalls::new(&every, FakeBlockActor::heap_delete).expect("every entry is there");
    assert!(calls.chest.is_some() && calls.skull.is_some());

    for missing in StratumEntry::BLOCK_ACTORS {
        let (lines, calls) = without(missing);

        assert!(calls.is_none(), "{}", missing.as_str());
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].starts_with(&format!("{} is off", StratumFeature::BLOCK_ACTORS)), "{lines:?}");
    }
}

#[test]
fn a_missing_large_chest_entry_turns_only_the_large_chests_off() {
    for missing in StratumEntry::LARGE_CHESTS {
        let (lines, calls) = without(missing);

        let calls = calls.unwrap_or_else(|| panic!("{}: the block actors stay on", missing.as_str()));
        assert!(calls.chest.is_none(), "{}", missing.as_str());
        assert!(calls.skull.is_some(), "{}", missing.as_str());
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert!(lines[0].starts_with(&format!("{} is off", StratumFeature::LARGE_CHESTS)), "{lines:?}");
    }
}

#[test]
fn a_missing_skull_entry_turns_the_skulls_off_and_no_other_ghost_feature() {
    for missing in StratumEntry::SKULLS {
        let (lines, calls) = without(missing);

        let calls = calls.unwrap_or_else(|| panic!("{}: the block actors stay on", missing.as_str()));
        assert!(calls.skull.is_none(), "{}", missing.as_str());
        assert!(calls.chest.is_some(), "{}", missing.as_str());
        // the block type name is also read by the build, which goes off with the skulls
        let shared = if missing == StratumEntry::BLOCK_TYPE_FULL_NAME { 2 } else { 1 };
        assert_eq!(lines.len(), shared, "{lines:?}");
        assert!(lines[0].starts_with(&format!("{} is off", StratumFeature::SKULLS)), "{lines:?}");
    }
}
