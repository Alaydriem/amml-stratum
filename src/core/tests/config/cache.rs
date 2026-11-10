use stratum_core::config::CacheConfig;

#[test]
fn budget_holds_at_least_one_max_size_structure() {
    let floor = CacheConfig::MIN_BUDGET_MB * 1024 * 1024;
    assert_eq!(CacheConfig { budget_mb: 16 }.budget_bytes(), floor);
    assert_eq!(CacheConfig { budget_mb: 0 }.budget_bytes(), floor);
    assert_eq!(CacheConfig { budget_mb: 300 }.budget_bytes(), 300 * 1024 * 1024);
}
