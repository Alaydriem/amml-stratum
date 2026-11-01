use std::collections::BTreeSet;

use alaydriem_stratum::StratumMod;
use alaydriem_stratum::game::ClientLevel;
use alaydriem_stratum::types::{StratumEntry, StratumFeature};
use amml_game::{EntryId, EntryNeed, GameError};

use crate::support::FakeResolver;

#[test]
fn with_every_entry_no_feature_is_off() {
    let entries = StratumMod::resolve(&FakeResolver::new(&FakeResolver::values())).unwrap();

    assert!(StratumMod::off_lines(&entries).is_empty());
}

#[test]
fn every_entry_is_asked_for_once() {
    let ids: Vec<EntryId> = StratumMod::wanted().iter().map(|(id, _)| *id).collect();

    for (index, id) in ids.iter().enumerate() {
        assert!(!ids[index + 1..].contains(id), "{} twice", id.as_str());
    }
}

#[test]
fn a_missing_connection_getter_turns_only_the_connection_off_with_one_line() {
    let values: Vec<_> = FakeResolver::values()
        .into_iter()
        .filter(|(id, _)| *id != StratumEntry::CLIENT_INSTANCE_GET_GAME_CONNECTION_INFO)
        .collect();

    let entries = StratumMod::resolve(&FakeResolver::new(&values)).unwrap();
    let lines = StratumMod::off_lines(&entries);

    assert_eq!(lines.len(), 1);
    assert!(lines[0].starts_with(&format!("{} is off", StratumFeature::CONNECTION)), "{lines:?}");
}

#[test]
fn a_missing_storage_constructor_turns_only_the_world_folder_off() {
    let values: Vec<_> =
        FakeResolver::values().into_iter().filter(|(id, _)| *id != StratumEntry::DB_STORAGE_CTOR).collect();

    let entries = StratumMod::resolve(&FakeResolver::new(&values)).unwrap();
    let lines = StratumMod::off_lines(&entries);

    assert_eq!(lines.len(), 1);
    assert!(lines[0].starts_with(&format!("{} is off", StratumFeature::WORLD_FOLDER)), "{lines:?}");
}

#[test]
fn a_missing_world_key_or_status_entry_refuses_the_enable() {
    for missing in [StratumEntry::LEVEL_GET_LEVEL_SEED64, StratumEntry::MINECRAFT_GAME_ON_APP_FOCUS_LOST] {
        let values: Vec<_> = FakeResolver::values().into_iter().filter(|(id, _)| *id != missing).collect();

        let result = StratumMod::resolve(&FakeResolver::new(&values));

        assert!(matches!(result, Err(GameError::MissingEntry { id }) if id == missing.as_str()));
    }
}

#[test]
fn a_level_name_anchor_that_does_not_fit_is_refused() {
    let entries =
        StratumMod::resolve(&FakeResolver::new(&FakeResolver::with(StratumEntry::LEVEL_DATA_LEVEL_NAME, 0x2B8)))
            .unwrap();
    let fitting = StratumMod::resolve(&FakeResolver::new(&FakeResolver::values())).unwrap();

    assert!(ClientLevel::check(&entries).is_err());
    assert!(ClientLevel::check(&fitting).is_ok());
}

#[test]
fn a_2660_level_name_anchor_fits_the_second_mirror() {
    let entries =
        StratumMod::resolve(&FakeResolver::new(&FakeResolver::with(StratumEntry::LEVEL_DATA_LEVEL_NAME, 0x2B0)))
            .unwrap();

    assert!(ClientLevel::check(&entries).is_ok());
}

#[test]
fn a_missing_ghost_entry_refuses_the_enable() {
    let values: Vec<_> =
        FakeResolver::values().into_iter().filter(|(id, _)| *id != StratumEntry::TESSELLATOR_END).collect();

    let result = StratumMod::resolve(&FakeResolver::new(&values));

    assert!(matches!(result, Err(GameError::MissingEntry { id }) if id == "Tessellator.end"));
}

#[test]
fn amml_hcl_lists_every_entry_the_mod_asks_for() {
    let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/amml.hcl")).unwrap();
    let manifest: hcl::Value = hcl::from_str(&text).unwrap();
    let entries = manifest.as_object().and_then(|object| object.get("entries")).and_then(hcl::Value::as_array).unwrap();
    let listed: BTreeSet<String> = entries.iter().map(|entry| entry.as_str().unwrap().to_string()).collect();
    let wanted: BTreeSet<String> = StratumMod::wanted().iter().map(|(id, _)| id.as_str().to_string()).collect();

    assert_eq!(listed, wanted);
}

#[test]
fn with_every_entry_the_easy_place_calls_accept_them_and_no_feature_is_named_off() {
    let entries = StratumMod::resolve(&FakeResolver::new(&FakeResolver::values())).unwrap();

    assert!(StratumMod::easy_place_check(&entries).is_ok());
    assert_eq!(StratumMod::features_line(&entries), "features off: none");
}

#[test]
fn a_missing_easy_place_entry_refuses_the_enable() {
    for missing in [StratumEntry::GAME_MODE_BUILD_BLOCK, StratumEntry::PACKET_READ, EntryId::ACTOR_GET_ATTACH_POS] {
        let values: Vec<_> = FakeResolver::values().into_iter().filter(|(id, _)| *id != missing).collect();

        let result = StratumMod::resolve(&FakeResolver::new(&values));

        assert!(
            matches!(result, Err(GameError::MissingEntry { ref id }) if id == missing.as_str()),
            "{}",
            missing.as_str()
        );
    }
}

#[test]
fn every_easy_place_entry_is_required() {
    let wanted = StratumMod::wanted();

    for id in StratumEntry::EASY_PLACE {
        let need = wanted.iter().find(|(wanted, _)| *wanted == id).map(|(_, need)| *need);
        assert_eq!(need, Some(EntryNeed::Required), "{}", id.as_str());
    }
}

#[test]
fn a_game_mode_mirror_that_does_not_fit_refuses_the_enable() {
    let entries =
        StratumMod::resolve(&FakeResolver::new(&FakeResolver::with(StratumEntry::GAME_MODE_PLAYER, 0x10))).unwrap();

    let error = StratumMod::easy_place_check(&entries).expect_err("the use calls refuse the mirror");

    assert!(error.to_string().contains(StratumEntry::GAME_MODE_PLAYER.as_str()), "{error}");
    assert!(StratumMod::off_lines(&entries).is_empty());
}
