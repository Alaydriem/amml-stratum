use stratum_core::config::{ImportConfig, ImportSettings};

#[test]
fn queue_is_clamped_to_one_through_1024() {
    let settings = |queue| ImportSettings { queue, ..ImportSettings::default() };
    assert_eq!(ImportConfig::from_settings(&settings(0)).queue, 1);
    assert_eq!(ImportConfig::from_settings(&settings(usize::MAX)).queue, 1024);
    assert_eq!(ImportConfig::from_settings(&settings(64)).queue, 64);
}

#[test]
fn megabytes_become_bytes() {
    let settings = ImportSettings { max_file_mb: 3, ..ImportSettings::default() };
    assert_eq!(ImportConfig::from_settings(&settings).max_file_bytes, 3 * 1024 * 1024);
}
