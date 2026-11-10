use stratum_core::services::preview::BrArchive;

use crate::support::PreviewFixture;

#[test]
fn entries_point_at_their_bytes_after_the_records() {
    let bytes = PreviewFixture::archive(&[("stone.png", vec![1, 2, 3]), ("leaves_oak.tga", vec![9, 8])]);
    let entries = BrArchive::entries(&bytes).unwrap();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].name, "stone.png");
    assert_eq!(entries[0].bytes(&bytes), Some(&[1u8, 2, 3][..]));
    assert_eq!(entries[1].name, "leaves_oak.tga");
    assert_eq!(entries[1].bytes(&bytes), Some(&[9u8, 8][..]));
}

#[test]
fn wrong_magic_or_short_records_are_not_an_archive() {
    let mut bytes = PreviewFixture::archive(&[("stone.png", vec![1])]);
    let short = bytes[..100].to_vec();
    assert!(BrArchive::entries(&short).is_none());
    bytes[0] = 0;
    assert!(BrArchive::entries(&bytes).is_none());
}

#[test]
fn an_entry_past_the_end_has_no_bytes() {
    let mut bytes = PreviewFixture::archive(&[("stone.png", vec![1, 2, 3])]);
    bytes.truncate(bytes.len() - 1);
    let entries = BrArchive::entries(&bytes).unwrap();
    assert_eq!(entries[0].bytes(&bytes), None);
}
