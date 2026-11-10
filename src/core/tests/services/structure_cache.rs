use std::path::{Path, PathBuf};
use std::sync::Arc;

use stratum_core::errors::StructureError;
use stratum_core::services::{StructureCache, StructureParser};
use stratum_core::structs::library::StructureId;

use crate::support::{ControllerHarness, StructureFixture};

// Fixture files for cache tests, all of the same size.
struct CacheFiles;

impl CacheFiles {
    fn write(dir: &Path, name: &str) -> PathBuf {
        ControllerHarness::picked_file(dir, name, &StructureFixture::full([4, 4, 4]).bytes())
    }

    // The bytes the cache counts for one fixture.
    fn one_fixture_bytes(dir: &Path) -> u64 {
        let mut probe = StructureCache::new(u64::MAX);
        probe.get_or_load(StructureId(99), &Self::write(dir, "probe.mcstructure")).unwrap();
        probe.resident_bytes()
    }
}

#[test]
fn cache_evicts_least_recently_used_past_budget() {
    let dir = tempfile::tempdir().unwrap();
    let one = CacheFiles::one_fixture_bytes(dir.path());
    let (a, b, c) = (StructureId(1), StructureId(2), StructureId(3));
    let pa = CacheFiles::write(dir.path(), "a.mcstructure");
    let pb = CacheFiles::write(dir.path(), "b.mcstructure");
    let pc = CacheFiles::write(dir.path(), "c.mcstructure");
    let budget = one * 2 + one / 2;
    let mut cache = StructureCache::new(budget);

    let first_a = cache.get_or_load(a, &pa).unwrap();
    cache.get_or_load(b, &pb).unwrap();
    assert_eq!(cache.resident_bytes(), one * 2);
    // Reading A makes B the least recently used.
    let second_a = cache.get_or_load(a, &pa).unwrap();
    assert!(Arc::ptr_eq(&first_a, &second_a));
    cache.get_or_load(c, &pc).unwrap();
    assert!(cache.resident_bytes() <= budget);
    assert_eq!(cache.resident_bytes(), one * 2);

    // With the files gone, only entries still in memory load.
    std::fs::remove_file(&pa).unwrap();
    std::fs::remove_file(&pb).unwrap();
    assert!(Arc::ptr_eq(&first_a, &cache.get_or_load(a, &pa).unwrap()));
    assert!(matches!(cache.get_or_load(b, &pb), Err(StructureError::Unreadable(_))));
}

#[test]
fn a_structure_bigger_than_the_budget_is_returned_but_not_kept() {
    let dir = tempfile::tempdir().unwrap();
    let one = CacheFiles::one_fixture_bytes(dir.path());
    let mut cache = StructureCache::new(one - 1);
    let loaded = cache.get_or_load(StructureId(1), &CacheFiles::write(dir.path(), "a.mcstructure")).unwrap();
    assert_eq!(loaded.size.x, 4);
    assert_eq!(cache.resident_bytes(), 0);
}

#[test]
fn evict_frees_the_entry() {
    let dir = tempfile::tempdir().unwrap();
    let path = CacheFiles::write(dir.path(), "a.mcstructure");
    let mut cache = StructureCache::new(u64::MAX);
    cache.get_or_load(StructureId(1), &path).unwrap();
    assert!(cache.resident_bytes() > 0);
    cache.evict(StructureId(1));
    assert_eq!(cache.resident_bytes(), 0);
    // Evicting an id that is not cached changes nothing.
    cache.evict(StructureId(7));
    assert_eq!(cache.resident_bytes(), 0);
}

#[test]
fn a_missing_or_broken_file_is_an_error_and_not_cached() {
    let dir = tempfile::tempdir().unwrap();
    let mut cache = StructureCache::new(u64::MAX);
    let missing = dir.path().join("none.mcstructure");
    assert!(matches!(cache.get_or_load(StructureId(1), &missing), Err(StructureError::Unreadable(_))));
    let broken = ControllerHarness::picked_file(dir.path(), "bad.mcstructure", b"not nbt");
    assert!(matches!(cache.get_or_load(StructureId(2), &broken), Err(StructureError::Nbt(_))));
    assert_eq!(cache.resident_bytes(), 0);
}

#[test]
fn max_size_structure_resident_bytes_under_56_mib() {
    let bytes = StructureFixture::full([256, 384, 256]).bytes();
    let structure = Arc::new(StructureParser::parse(&bytes).unwrap());
    drop(bytes);
    let mut cache = StructureCache::new(64 * 1024 * 1024);
    cache.insert(StructureId(1), Arc::clone(&structure));

    // 25,165,824 cells at 2 bytes each is 48 MiB.
    assert!(cache.resident_bytes() >= 48 * 1024 * 1024);
    assert!(cache.resident_bytes() < 56 * 1024 * 1024);
    assert!(Arc::ptr_eq(&structure, &cache.get(StructureId(1)).unwrap()));
}

#[test]
fn charged_bytes_count_against_the_budget_and_evict_entries() {
    let dir = tempfile::tempdir().unwrap();
    let one = CacheFiles::one_fixture_bytes(dir.path());
    let mut cache = StructureCache::new(one * 2);
    cache.get_or_load(StructureId(1), &CacheFiles::write(dir.path(), "a.mcstructure")).unwrap();
    cache.get_or_load(StructureId(2), &CacheFiles::write(dir.path(), "b.mcstructure")).unwrap();
    assert_eq!(cache.resident_bytes(), one * 2);

    cache.set_charged(one / 2);

    assert_eq!(cache.charged_bytes(), one / 2);
    assert_eq!(cache.resident_bytes(), one + one / 2);
    assert!(cache.resident_bytes() <= cache.budget_bytes());

    cache.set_charged(0);
    assert_eq!(cache.resident_bytes(), one);
}
