use alaydriem_stratum::StratumMod;
use alaydriem_stratum::game::MeshLayoutKind;
use alaydriem_stratum::types::StratumEntry;

use crate::support::FakeResolver;

fn kind(size: u64) -> Option<MeshLayoutKind> {
    let entries = StratumMod::resolve(&FakeResolver::new(&FakeResolver::with(StratumEntry::MESH_SIZE, size))).unwrap();
    MeshLayoutKind::of(&entries)
}

#[test]
fn the_mesh_size_selects_the_mirror_of_its_build() {
    assert_eq!(kind(632), Some(MeshLayoutKind::Build2651));
    assert_eq!(kind(608), Some(MeshLayoutKind::Build2660));
}

#[test]
fn a_mesh_size_that_no_mirror_has_selects_none() {
    assert_eq!(kind(640), None);
}
