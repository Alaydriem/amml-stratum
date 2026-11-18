use alaydriem_stratum::build::KindsScanner;
use stratum_core::structs::build::{BuildKindsRequest, BuildKindsResult};

use crate::support::FakeBuildWorld;

fn request(min: [i32; 3], max: [i32; 3]) -> BuildKindsRequest {
    BuildKindsRequest { min, max }
}

fn named_world() -> FakeBuildWorld {
    let mut world = FakeBuildWorld::ready();
    world.names.insert(1, "minecraft:stone".to_string());
    world.names.insert(2, "minecraft:water".to_string());
    world.names.insert(3, "minecraft:lava".to_string());
    world
}

fn sorted(result: Option<BuildKindsResult>) -> Vec<String> {
    match result {
        Some(BuildKindsResult::Kinds(mut kinds)) => {
            kinds.sort();
            kinds
        }
        other => panic!("not a kinds answer: {other:?}"),
    }
}

#[test]
fn scan_spreads_over_ticks_and_reports_kinds() {
    let mut world = named_world();
    world.fill([0, 0, 0], [3, 3, 3], 1);
    world.cells.insert([2, 1, 3], 2);
    world.cells.insert([3, 3, 0], 3);
    let mut scanner = KindsScanner::new(10);
    scanner.begin(&request([0, 0, 0], [3, 3, 3]));

    let mut ticks = 0;
    let answer = loop {
        ticks += 1;
        if let Some(answer) = scanner.step(&world) {
            break answer;
        }
        assert!(ticks < 20, "the scan does not end");
    };

    // 64 cells at 10 a tick
    assert_eq!(ticks, 7);
    assert_eq!(sorted(Some(answer)), vec!["minecraft:lava", "minecraft:stone", "minecraft:water"]);
    assert!(!scanner.is_scanning());
}

#[test]
fn one_tick_reads_no_more_cells_than_its_rate() {
    let mut world = named_world();
    world.fill([0, 0, 0], [9, 9, 9], 1);
    let mut scanner = KindsScanner::new(25);
    scanner.begin(&request([0, 0, 0], [9, 9, 9]));

    assert!(scanner.step(&world).is_none());

    assert_eq!(world.cell_reads.get(), 25);
}

#[test]
fn each_kind_is_reported_once_however_many_cells_hold_it() {
    let mut world = named_world();
    world.fill([0, 0, 0], [4, 4, 4], 1);
    let mut scanner = KindsScanner::new(1000);
    scanner.begin(&request([0, 0, 0], [4, 4, 4]));

    let kinds = sorted(scanner.step(&world));

    assert_eq!(kinds, vec!["minecraft:stone"]);
}

#[test]
fn scan_reports_not_loaded() {
    let mut world = named_world();
    world.fill([0, 0, 0], [31, 3, 3], 1);
    world.unloaded_chunks.insert((1, 0));
    let mut scanner = KindsScanner::new(1_000_000);
    scanner.begin(&request([0, 0, 0], [31, 3, 3]));

    assert_eq!(scanner.step(&world), Some(BuildKindsResult::NotLoaded));
    assert!(!scanner.is_scanning());
}

#[test]
fn a_block_that_reads_null_is_not_loaded() {
    let mut world = named_world();
    world.fill([0, 0, 0], [3, 3, 3], 1);
    world.cells.remove(&[2, 2, 2]);
    let mut scanner = KindsScanner::new(1_000_000);
    scanner.begin(&request([0, 0, 0], [3, 3, 3]));

    assert_eq!(scanner.step(&world), Some(BuildKindsResult::NotLoaded));
}

#[test]
fn a_kind_whose_name_cannot_be_read_is_not_loaded() {
    let mut world = named_world();
    world.fill([0, 0, 0], [1, 1, 1], 9);
    let mut scanner = KindsScanner::new(1_000_000);
    scanner.begin(&request([0, 0, 0], [1, 1, 1]));

    assert_eq!(scanner.step(&world), Some(BuildKindsResult::NotLoaded));
}

#[test]
fn negative_cells_are_checked_in_their_own_chunk() {
    let mut world = named_world();
    world.fill([-2, 0, -2], [-1, 0, -1], 1);
    world.unloaded_chunks.insert((-1, -1));
    let mut scanner = KindsScanner::new(1_000_000);
    scanner.begin(&request([-2, 0, -2], [-1, 0, -1]));

    assert_eq!(scanner.step(&world), Some(BuildKindsResult::NotLoaded));
}

#[test]
fn more_kinds_than_the_bound_is_too_many_and_the_list_stays_inside_the_bound() {
    let mut world = FakeBuildWorld::ready();
    let kinds = KindsScanner::KIND_BOUND + 5;
    for kind in 0..kinds {
        world.names.insert(kind, format!("minecraft:block_{kind}"));
        world.cells.insert([i32::try_from(kind).unwrap(), 0, 0], kind);
    }
    let last = i32::try_from(kinds).unwrap() - 1;
    let mut scanner = KindsScanner::new(1);
    scanner.begin(&request([0, 0, 0], [last, 0, 0]));

    let mut most = 0;
    let answer = loop {
        let answer = scanner.step(&world);
        most = most.max(scanner.found());
        if answer.is_some() {
            break answer;
        }
    };

    assert_eq!(answer, Some(BuildKindsResult::TooMany));
    assert_eq!(most, KindsScanner::KIND_BOUND, "the list filled to its bound and no further");
    assert_eq!(scanner.found(), 0, "the list is dropped with the answer");
}

#[test]
fn a_new_request_replaces_a_scan_that_is_running() {
    let mut world = named_world();
    world.fill([0, 0, 0], [9, 9, 9], 1);
    world.fill([20, 0, 0], [20, 0, 0], 2);
    let mut scanner = KindsScanner::new(5);
    scanner.begin(&request([0, 0, 0], [9, 9, 9]));
    assert!(scanner.step(&world).is_none());

    scanner.begin(&request([20, 0, 0], [20, 0, 0]));

    assert_eq!(sorted(scanner.step(&world)), vec!["minecraft:water"]);
}

#[test]
fn a_scan_that_was_not_begun_answers_nothing() {
    let world = named_world();
    let mut scanner = KindsScanner::new(5);

    assert_eq!(scanner.step(&world), None);
}

#[test]
fn a_chunk_that_unloads_between_ticks_is_not_loaded() {
    let mut world = named_world();
    world.fill([0, 0, 0], [3, 3, 3], 1);
    let mut scanner = KindsScanner::new(10);
    scanner.begin(&request([0, 0, 0], [3, 3, 3]));
    assert!(scanner.step(&world).is_none());

    world.unloaded_chunks.insert((0, 0));

    assert_eq!(scanner.step(&world), Some(BuildKindsResult::NotLoaded));
}
