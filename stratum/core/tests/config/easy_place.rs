use stratum_core::config::{EasyPlaceConfig, StratumConfig};

#[test]
fn zero_capacities_are_raised_to_one() {
    let config = EasyPlaceConfig { pending_capacity: 0, tried_capacity: 0, ..EasyPlaceConfig::default() };

    assert_eq!(config.pending_capacity(), 1);
    assert_eq!(config.tried_capacity(), 1);
}

#[test]
fn an_absent_block_gives_the_defaults() {
    let config: StratumConfig = hcl::from_str(
        "ghost {
  draw_distance = 6
}
",
    )
    .unwrap();

    assert_eq!(config.easy_place, EasyPlaceConfig::default());
}
