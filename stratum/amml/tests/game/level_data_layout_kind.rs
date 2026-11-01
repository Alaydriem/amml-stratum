use alaydriem_stratum::StratumMod;
use alaydriem_stratum::game::LevelDataLayoutKind;
use alaydriem_stratum::types::StratumEntry;

use crate::support::FakeResolver;

fn kind(offset: u64) -> Option<LevelDataLayoutKind> {
    let values = FakeResolver::with(StratumEntry::LEVEL_DATA_LEVEL_NAME, offset);
    let entries = StratumMod::resolve(&FakeResolver::new(&values)).unwrap();
    LevelDataLayoutKind::of(&entries)
}

#[test]
fn the_level_name_offset_selects_the_mirror_of_its_build() {
    assert_eq!(kind(0x2A8), Some(LevelDataLayoutKind::Build2651));
    assert_eq!(kind(0x2B0), Some(LevelDataLayoutKind::Build2660));
}

#[test]
fn a_level_name_offset_that_no_mirror_has_selects_none() {
    assert_eq!(kind(0x2B8), None);
}
