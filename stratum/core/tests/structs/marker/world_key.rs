use stratum_core::structs::marker::WorldKey;

#[test]
fn world_key_file_stem_is_stable_and_safe() {
    let key = WorldKey { world: "My World: ÆØ/\\?".to_string(), dimension: 1 };
    let stem = key.file_stem();
    assert_eq!(stem, key.clone().file_stem());
    assert!(stem.ends_with("-1"));
    assert!(stem.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f' | '-')));
}

#[test]
fn file_stem_matches_known_fnv_1a_values() {
    // FNV-1a 64 of "a" is af63dc4c8601ec8c; of the empty text it is the offset basis.
    assert_eq!(WorldKey { world: "a".to_string(), dimension: 0 }.file_stem(), "af63dc4c8601ec8c-0");
    assert_eq!(WorldKey { world: String::new(), dimension: 2 }.file_stem(), "cbf29ce484222325-2");
}

#[test]
fn negative_dimension_keeps_its_sign() {
    let stem = WorldKey { world: "a".to_string(), dimension: -1 }.file_stem();
    assert_eq!(stem, "af63dc4c8601ec8c--1");
}

#[test]
fn dimensions_give_different_stems() {
    let nether = WorldKey { world: "w".to_string(), dimension: -1 };
    let end = WorldKey { world: "w".to_string(), dimension: 1 };
    assert_ne!(nether.file_stem(), end.file_stem());
}
