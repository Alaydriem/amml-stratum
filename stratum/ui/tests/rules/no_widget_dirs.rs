use amml_ui_rules::folder::ForbiddenFolders;

#[test]
fn no_widget_dirs() {
    let present = ForbiddenFolders::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src"), &["widgets", "theme"]).present();
    assert!(present.is_empty(), "the UI crate has widget or theme code: {present:?}");
}
