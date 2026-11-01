use stratum_core::config::{GhostConfig, StratumConfig};

#[test]
fn a_config_that_still_sets_the_old_opacity_loads() {
    let config: StratumConfig = hcl::from_str(
        "ghost {
  opacity = 80
  draw_distance = 6
}
",
    )
    .unwrap();

    assert_eq!(config.ghost.draw_distance, 6);
}

#[test]
fn a_config_without_the_miniature_field_shows_the_mini_ghost() {
    let config: StratumConfig = hcl::from_str(
        "ghost {
  draw_distance = 6
}
",
    )
    .unwrap();
    let off: StratumConfig = hcl::from_str(
        "ghost {
  miniature = false
}
",
    )
    .unwrap();

    assert!(config.ghost.miniature);
    assert!(!off.ghost.miniature);
}

#[test]
fn budgets_never_drop_below_one_section() {
    let empty_budget = GhostConfig { mesh_budget_mb: 0, ..GhostConfig::default() };
    let no_cells = GhostConfig { cells_per_frame: 0, ..GhostConfig::default() };

    assert!(empty_budget.mesh_budget_bytes() >= GhostConfig::MIN_BUDGET_BYTES);
    assert_eq!(no_cells.cells_per_frame(), GhostConfig::MIN_CELLS_PER_FRAME);
    assert_eq!(GhostConfig::default().mesh_budget_bytes(), 1024 * 1024 * 1024);
}

#[test]
fn zero_draw_distance_follows_the_game() {
    let follows = GhostConfig { draw_distance: 0, ..GhostConfig::default() };
    let fixed = GhostConfig { draw_distance: 6, ..GhostConfig::default() };

    assert_eq!(follows.draw_distance_chunks(12), 12);
    assert_eq!(fixed.draw_distance_chunks(12), 6);
}
