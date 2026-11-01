use stratum_ui::presenter::MarkerFilter;

#[test]
fn empty_filter_matches_all() {
    assert!(MarkerFilter::matches("Dock", "Harbour", ""));
    assert!(MarkerFilter::matches("Dock", "Harbour", "   "));
}

#[test]
fn filter_matches_the_target_name() {
    assert!(MarkerFilter::matches("Dock", "Harbour west", "west"));
    assert!(MarkerFilter::matches("Dock", "Harbour west", "doc"));
}

#[test]
fn filter_ignores_case() {
    assert!(MarkerFilter::matches("Dock", "Harbour", "DOCK"));
    assert!(MarkerFilter::matches("Dock", "Harbour", "hARb"));
}

#[test]
fn filter_trims_spaces() {
    assert!(MarkerFilter::matches("Dock", "Harbour", "  dock "));
}

#[test]
fn no_match_returns_false() {
    assert!(!MarkerFilter::matches("Dock", "Harbour", "lighthouse"));
    assert!(!MarkerFilter::matches("Dock", "", "x"));
}
