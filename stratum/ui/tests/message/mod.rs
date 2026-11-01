use std::path::Path;

use amml_i18n_tool::catalog::CatalogCheck;
use stratum_ui::message::StratumMessage;

#[test]
fn message_catalog() {
    let problems = CatalogCheck::problems::<StratumMessage>();
    assert!(problems.is_empty(), "{}", problems.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n"));
}

#[test]
fn pot_up_to_date() {
    let i18n = Path::new(env!("CARGO_MANIFEST_DIR")).join("i18n");
    let current = CatalogCheck::pot_is_current::<StratumMessage>(&i18n).unwrap_or_else(|error| panic!("{error}"));
    assert!(current, "stratum.pot is out of date: run `mise run i18n:extract`");
}
