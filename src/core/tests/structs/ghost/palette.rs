use stratum_core::errors::GhostError;
use stratum_core::structs::ghost::GhostPalette;
use stratum_core::structs::structure::{BlockState, BlockStates};

#[test]
fn marker_id_appends_once_and_reuses() {
    let mut palette = GhostPalette::new();
    palette.entries.push(BlockState::new("minecraft:stone".to_string(), BlockStates::default()));
    let first = palette.marker_id().unwrap();
    let second = palette.marker_id().unwrap();
    assert_eq!(first, 1);
    assert_eq!(second, 1);
    assert_eq!(palette.entries.len(), 2);
    assert_eq!(palette.entries[1], GhostPalette::marker_entry());
}

#[test]
fn marker_entry_counts_toward_the_palette_limit() {
    let mut palette = GhostPalette::new();
    palette.entries = (0..GhostPalette::MAX_ENTRIES)
        .map(|n| BlockState::new(format!("test:block_{n}"), BlockStates::default()))
        .collect();
    assert!(matches!(palette.marker_id(), Err(GhostError::PaletteTooLarge { .. })));
}
