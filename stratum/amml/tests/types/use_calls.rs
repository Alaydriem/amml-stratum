use alaydriem_stratum::StratumMod;
use alaydriem_stratum::errors::GameLayoutError;
use alaydriem_stratum::types::{StratumEntry, UseCalls};

use crate::support::FakeResolver;

#[test]
fn the_game_mode_anchor_where_the_mirror_has_m_player_is_taken() {
    let entries = StratumMod::resolve(&FakeResolver::new(&FakeResolver::values())).unwrap();

    assert_eq!(UseCalls::new(&entries).map(|calls| calls.game_mode_player), Ok(0x8));
}

#[test]
fn a_game_mode_anchor_off_the_mirror_is_refused() {
    let entries =
        StratumMod::resolve(&FakeResolver::new(&FakeResolver::with(StratumEntry::GAME_MODE_PLAYER, 0x10))).unwrap();

    assert_eq!(
        UseCalls::new(&entries).map(|calls| calls.game_mode_player),
        Err(GameLayoutError::Misfit { entry: "GameMode.mPlayer", expected: 0x8, found: 0x10 })
    );
}
