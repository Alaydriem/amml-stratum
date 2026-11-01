use stratum_core::services::preview::SourceLocator;

#[test]
fn the_packs_are_found_in_a_folder_above_the_data_folder() {
    let game = tempfile::tempdir().unwrap();
    let packs = game.path().join("data").join("resource_packs");
    std::fs::create_dir_all(packs.join("vanilla")).unwrap();
    std::fs::write(packs.join("vanilla").join("blocks.json"), "{}").unwrap();
    let data = game.path().join("mods").join("amml").join("@alaydriem").join("stratum").join("data");
    let source = SourceLocator::locate(&data);
    assert_eq!(source.packs_dir, Some(packs));
    assert_eq!(source.structures_dir, data.join("structures"));
}
