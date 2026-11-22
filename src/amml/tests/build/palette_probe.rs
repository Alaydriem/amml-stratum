use alaydriem_stratum::build::PaletteProbe;
use alaydriem_stratum::types::PaletteLookup;
use stratum_core::structs::build::{BuildProbeRequest, PaletteOutcome};

use crate::support::FakeBuildWorld;

fn entry(tag: u8) -> (Vec<u8>, Vec<u8>) {
    (vec![tag], vec![tag, 0xFF])
}

#[test]
fn probe_reports_exact_name_only_unknown_and_mismatch() {
    let mut world = FakeBuildWorld::ready();
    // 0: exact, turned the same way; 1: name only; 2: not in the palette; 3: exact, but the game turns it elsewhere
    world.lookups.insert(vec![0], PaletteLookup::Exact(FakeBuildWorld::block(0)));
    world.lookups.insert(vec![0, 0xFF], PaletteLookup::Exact(FakeBuildWorld::block(10)));
    world.turns.insert((FakeBuildWorld::block(0), 1), FakeBuildWorld::block(10));
    world.lookups.insert(vec![1], PaletteLookup::Default(FakeBuildWorld::block(1)));
    world.lookups.insert(vec![1, 0xFF], PaletteLookup::Default(FakeBuildWorld::block(1)));
    world.lookups.insert(vec![3], PaletteLookup::Exact(FakeBuildWorld::block(3)));
    world.lookups.insert(vec![3, 0xFF], PaletteLookup::Exact(FakeBuildWorld::block(13)));
    world.turns.insert((FakeBuildWorld::block(3), 1), FakeBuildWorld::block(99));
    let request = BuildProbeRequest { entries: vec![entry(0), entry(1), entry(2), entry(3)], quarters: 1 };

    let result = PaletteProbe::answer(&world, &request);

    assert_eq!(
        result.outcomes,
        vec![PaletteOutcome::Exact, PaletteOutcome::NameOnly, PaletteOutcome::Unknown, PaletteOutcome::Exact]
    );
    assert_eq!(result.turn_mismatches, vec![3]);
}

#[test]
fn an_empty_entry_is_unknown_and_is_never_looked_up() {
    let world = FakeBuildWorld::ready();
    let request = BuildProbeRequest { entries: vec![(Vec::new(), Vec::new())], quarters: 0 };

    let result = PaletteProbe::answer(&world, &request);

    assert_eq!(result.outcomes, vec![PaletteOutcome::Unknown]);
    assert!(result.turn_mismatches.is_empty());
    assert_eq!(world.lookup_calls.get(), 0);
}

#[test]
fn an_unknown_block_is_not_a_mismatch() {
    let world = FakeBuildWorld::ready();
    let request = BuildProbeRequest { entries: vec![entry(7)], quarters: 2 };

    let result = PaletteProbe::answer(&world, &request);

    assert_eq!(result.outcomes, vec![PaletteOutcome::Unknown]);
    assert!(result.turn_mismatches.is_empty());
}

#[test]
fn the_turn_is_asked_with_the_quarters_of_the_request() {
    let mut world = FakeBuildWorld::ready();
    world.lookups.insert(vec![0], PaletteLookup::Exact(FakeBuildWorld::block(0)));
    world.lookups.insert(vec![0, 0xFF], PaletteLookup::Exact(FakeBuildWorld::block(5)));
    // the game turns block 0 into block 5 by three quarters only
    world.turns.insert((FakeBuildWorld::block(0), 3), FakeBuildWorld::block(5));

    let right = PaletteProbe::answer(&world, &BuildProbeRequest { entries: vec![entry(0)], quarters: 3 });
    let wrong = PaletteProbe::answer(&world, &BuildProbeRequest { entries: vec![entry(0)], quarters: 1 });

    assert!(right.turn_mismatches.is_empty());
    assert_eq!(wrong.turn_mismatches, vec![0]);
}
