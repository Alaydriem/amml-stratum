use stratum_core::services::BuildSettingsStore;
use stratum_core::structs::build::{BuildSettings, BuildSettingsBook};
use stratum_core::structs::library::StructureId;

fn rate(blocks_per_second: u8) -> BuildSettings {
    BuildSettings { blocks_per_second, ..BuildSettings::default() }
}

fn ids(book: &BuildSettingsBook) -> Vec<u64> {
    book.entries().iter().map(|(id, _)| id.0).collect()
}

#[test]
fn settings_book_saves_clamps_and_evicts() {
    let dir = tempfile::tempdir().unwrap();
    let store = BuildSettingsStore::new(dir.path().to_path_buf());

    // A rate below and a rate above the range are saved as given and clamped when the file loads.
    let mut book = BuildSettingsBook::new(8);
    book.set(StructureId(1), rate(0));
    book.set(StructureId(2), rate(200));
    book.set(StructureId(3), BuildSettings { blocks_per_second: 7, clear_water: false, clear_lava: true });
    store.save(&book).unwrap();

    let loaded = store.load(8).unwrap();
    assert_eq!(loaded.get(StructureId(1)).blocks_per_second, BuildSettings::MIN_RATE);
    assert_eq!(loaded.get(StructureId(2)).blocks_per_second, BuildSettings::MAX_RATE);
    assert_eq!(
        loaded.get(StructureId(3)),
        BuildSettings { blocks_per_second: 7, clear_water: false, clear_lava: true }
    );

    // A smaller capacity at load time drops the entries used longest ago.
    let small = store.load(2).unwrap();
    assert_eq!(ids(&small), vec![2, 3]);

    // A structure with no entry reads as the defaults.
    assert_eq!(loaded.get(StructureId(9)), BuildSettings::default());
}

#[test]
fn the_oldest_used_entry_is_evicted_at_capacity() {
    let mut book = BuildSettingsBook::new(2);
    book.set(StructureId(1), rate(5));
    book.set(StructureId(2), rate(6));
    // Reading the first entry makes the second the oldest.
    assert_eq!(book.use_settings(StructureId(1)).blocks_per_second, 5);

    book.set(StructureId(3), rate(7));

    assert_eq!(book.len(), 2);
    assert_eq!(ids(&book), vec![1, 3]);
    assert_eq!(book.get(StructureId(2)), BuildSettings::default());
}

#[test]
fn setting_a_structure_again_replaces_its_entry_without_growing_the_book() {
    let mut book = BuildSettingsBook::new(2);
    book.set(StructureId(1), rate(5));
    book.set(StructureId(2), rate(6));

    book.set(StructureId(1), rate(9));

    assert_eq!(ids(&book), vec![2, 1]);
    assert_eq!(book.get(StructureId(1)).blocks_per_second, 9);
}

#[test]
fn a_removed_entry_is_gone_and_marks_the_book_dirty() {
    let mut book = BuildSettingsBook::new(4);
    book.set(StructureId(1), rate(5));
    book.mark_saved();

    book.remove(StructureId(2));
    assert!(!book.is_dirty());
    book.remove(StructureId(1));

    assert!(book.is_empty());
    assert!(book.is_dirty());
}

#[test]
fn a_corrupt_settings_file_is_kept_and_an_empty_book_loads() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("build.hcl"), "settings = [ { structure = ").unwrap();
    let store = BuildSettingsStore::new(dir.path().to_path_buf());

    let book = store.load(4).unwrap();

    assert!(book.is_empty());
    let names: Vec<String> = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(names.iter().any(|name| name.starts_with("build.hcl.bad-")));
    assert!(!names.iter().any(|name| name == "build.hcl"));
}

#[test]
fn an_owed_restore_is_added_once_cleared_and_marks_the_book_dirty() {
    let mut book = BuildSettingsBook::new(4);
    assert!(!book.owes("W1"));

    book.owe("W1");
    book.owe("W1");

    assert!(book.owes("W1"));
    assert_eq!(book.owed(), ["W1".to_string()]);
    assert!(book.is_dirty());
    book.mark_saved();
    book.owe("W1");
    assert!(!book.is_dirty(), "owing again changes nothing");

    book.clear_owed("W2");
    assert!(!book.is_dirty());
    book.clear_owed("W1");
    assert!(!book.owes("W1"));
    assert!(book.is_dirty());
}

#[test]
fn the_owed_restores_are_bounded_and_the_oldest_goes_first() {
    let mut book = BuildSettingsBook::new(4);
    for index in 0..BuildSettingsBook::OWED_CAPACITY + 5 {
        book.owe(&format!("world {index}"));
    }

    assert_eq!(book.owed().len(), BuildSettingsBook::OWED_CAPACITY);
    assert!(!book.owes("world 0"));
    assert!(book.owes(&format!("world {}", BuildSettingsBook::OWED_CAPACITY + 4)));
}

#[test]
fn an_owed_restore_is_saved_and_loaded_and_an_old_file_without_it_still_loads() {
    let dir = tempfile::tempdir().unwrap();
    let store = BuildSettingsStore::new(dir.path().to_path_buf());
    let mut book = BuildSettingsBook::new(4);
    book.set(StructureId(1), rate(5));
    book.owe("My World");
    book.owe("W2");
    store.save(&book).unwrap();

    let loaded = store.load(4).unwrap();

    assert_eq!(loaded.owed(), ["My World".to_string(), "W2".to_string()]);
    assert_eq!(loaded.get(StructureId(1)).blocks_per_second, 5);

    // A file written before the field existed.
    std::fs::write(dir.path().join("build.hcl"), "settings = [{ structure = 1, blocks_per_second = 9 }]\n").unwrap();
    let old = store.load(4).unwrap();
    assert!(old.owed().is_empty());
    assert_eq!(old.get(StructureId(1)).blocks_per_second, 9);
}

#[test]
fn a_file_with_too_many_owed_worlds_loads_the_newest() {
    let dir = tempfile::tempdir().unwrap();
    let worlds: Vec<String> = (0..BuildSettingsBook::OWED_CAPACITY + 3).map(|index| format!("\"w{index}\"")).collect();
    std::fs::write(dir.path().join("build.hcl"), format!("owed = [{}]\n", worlds.join(", "))).unwrap();

    let book = BuildSettingsStore::new(dir.path().to_path_buf()).load(4).unwrap();

    assert_eq!(book.owed().len(), BuildSettingsBook::OWED_CAPACITY);
    assert!(!book.owes("w0"));
    assert!(book.owes(&format!("w{}", BuildSettingsBook::OWED_CAPACITY + 2)));
}
