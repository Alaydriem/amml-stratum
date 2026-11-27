use alaydriem_stratum::build::KindsScanner;
use stratum_core::structs::build::{BuildKindsRequest, BuildKindsResult};

use crate::support::FakeBuildWorld;

const STONE: &str = "minecraft:stone";
const WATER: &str = "minecraft:water";
const LAVA: &str = "minecraft:lava";

fn request(min: [i32; 3], max: [i32; 3]) -> BuildKindsRequest {
    BuildKindsRequest { min, max }
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

fn run(scanner: &mut KindsScanner, world: &FakeBuildWorld) -> (BuildKindsResult, u32) {
    let mut ticks = 0;
    loop {
        ticks += 1;
        if let Some(answer) = scanner.step(world) {
            return (answer, ticks);
        }
        assert!(ticks < 1000, "the scan does not end");
    }
}

#[test]
fn scan_spreads_over_ticks_in_slabs_and_reports_kinds() {
    let mut world = FakeBuildWorld::ready();
    world.fill([0, 0, 0], [3, 3, 3], STONE);
    world.cells.insert([2, 1, 3], WATER.to_string());
    world.cells.insert([3, 3, 0], LAVA.to_string());
    // a slab of one column of 4 cells and a row of 2 columns: 8 cells
    let mut scanner = KindsScanner::new(10);
    scanner.begin(&request([0, 0, 0], [3, 3, 3]));

    let (answer, ticks) = run(&mut scanner, &world);

    assert_eq!(ticks, 8);
    assert_eq!(sorted(Some(answer)), vec![LAVA, STONE, WATER]);
    assert!(!scanner.is_scanning());
}

#[test]
fn no_slab_is_larger_than_the_rate_or_the_loader_cap() {
    let mut world = FakeBuildWorld::ready();
    world.fill([0, 0, 0], [9, 9, 9], STONE);
    let mut scanner = KindsScanner::new(25);
    scanner.begin(&request([0, 0, 0], [9, 9, 9]));
    run(&mut scanner, &world);

    let mut big = KindsScanner::new(u32::MAX);
    big.begin(&request([0, -64, 0], [63, 319, 63]));
    let tall = FakeBuildWorld::ready();
    run(&mut big, &tall);

    for (min, max) in world.boxes.borrow().iter().chain(tall.boxes.borrow().iter()) {
        let cells: i64 = (0..3).map(|axis| i64::from(max[axis] - min[axis] + 1)).product();
        assert!(cells <= 65_536, "a slab of {cells} cells");
    }
    assert!(
        world
            .boxes
            .borrow()
            .iter()
            .all(|(min, max)| (0..3).map(|axis| i64::from(max[axis] - min[axis] + 1)).product::<i64>() <= 25)
    );
}

#[test]
fn the_slabs_cover_the_box_once() {
    let mut world = FakeBuildWorld::ready();
    world.fill([0, 0, 0], [6, 4, 5], STONE);
    let mut scanner = KindsScanner::new(9);
    scanner.begin(&request([0, 0, 0], [6, 4, 5]));
    run(&mut scanner, &world);

    let mut seen = std::collections::HashMap::new();
    for (min, max) in world.boxes.borrow().iter() {
        for x in min[0]..=max[0] {
            for y in min[1]..=max[1] {
                for z in min[2]..=max[2] {
                    *seen.entry([x, y, z]).or_insert(0) += 1;
                }
            }
        }
    }
    assert_eq!(seen.len(), 7 * 5 * 6);
    assert!(seen.values().all(|count| *count == 1));
}

#[test]
fn each_kind_is_reported_once_however_many_slabs_hold_it() {
    let mut world = FakeBuildWorld::ready();
    world.fill([0, 0, 0], [4, 4, 4], STONE);
    let mut scanner = KindsScanner::new(5);
    scanner.begin(&request([0, 0, 0], [4, 4, 4]));

    let (answer, _) = run(&mut scanner, &world);

    assert_eq!(sorted(Some(answer)), vec![STONE]);
}

#[test]
fn scan_reports_not_loaded() {
    let mut world = FakeBuildWorld::ready();
    world.fill([0, 0, 0], [31, 3, 3], STONE);
    world.unloaded_chunks.insert((1, 0));
    let mut scanner = KindsScanner::new(65_536);
    scanner.begin(&request([0, 0, 0], [31, 3, 3]));

    let (answer, _) = run(&mut scanner, &world);

    assert_eq!(answer, BuildKindsResult::NotLoaded);
    assert!(!scanner.is_scanning());
}

#[test]
fn scan_reports_too_many_past_the_kind_bound() {
    let mut world = FakeBuildWorld::ready();
    let count = KindsScanner::KIND_BOUND as i32 + 1;
    for x in 0..count {
        world.cells.insert([x, 0, 0], format!("minecraft:kind_{x}"));
    }
    let mut scanner = KindsScanner::new(4096);
    scanner.begin(&request([0, 0, 0], [count - 1, 0, 0]));

    let (answer, _) = run(&mut scanner, &world);

    assert_eq!(answer, BuildKindsResult::TooMany);
}

#[test]
fn an_inverted_box_has_no_kinds() {
    let world = FakeBuildWorld::ready();
    let mut scanner = KindsScanner::new(100);
    scanner.begin(&request([3, 0, 0], [0, 0, 0]));

    assert_eq!(scanner.step(&world), Some(BuildKindsResult::Kinds(Vec::new())));
    assert!(world.boxes.borrow().is_empty());
}
