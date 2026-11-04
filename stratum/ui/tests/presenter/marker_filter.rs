use stratum_ui::presenter::MarkerFilter;

#[test]
fn empty_filter_matches_all() {
    assert!(MarkerFilter::matches("Dock", "Harbour", None, ""));
    assert!(MarkerFilter::matches("Dock", "Harbour", None, "   "));
}

#[test]
fn filter_matches_the_target_name() {
    assert!(MarkerFilter::matches("Dock", "Harbour west", None, "west"));
    assert!(MarkerFilter::matches("Dock", "Harbour west", None, "doc"));
}

#[test]
fn filter_ignores_case() {
    assert!(MarkerFilter::matches("Dock", "Harbour", None, "DOCK"));
    assert!(MarkerFilter::matches("Dock", "Harbour", None, "hARb"));
}

#[test]
fn filter_trims_spaces() {
    assert!(MarkerFilter::matches("Dock", "Harbour", None, "  dock "));
}

#[test]
fn no_match_returns_false() {
    assert!(!MarkerFilter::matches("Dock", "Harbour", None, "lighthouse"));
    assert!(!MarkerFilter::matches("Dock", "", None, "x"));
}

#[test]
fn marker_filter_matches_the_part_name() {
    assert!(MarkerFilter::matches("Dock", "Harbour", Some("Bonsai leaves"), "leaves"));
    assert!(!MarkerFilter::matches("Dock", "Harbour", None, "leaves"));
    assert!(!MarkerFilter::matches("Dock", "Harbour", Some("Bonsai bark"), "leaves"));
}
