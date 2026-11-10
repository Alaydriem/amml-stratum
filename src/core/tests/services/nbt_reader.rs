use stratum_core::errors::NbtError;
use stratum_core::services::NbtReader;
use stratum_core::structs::nbt::{NbtLimits, NbtTag};

use crate::support::{NbtFixtures, NbtWriter};

#[test]
fn reads_every_tag_type() {
    let bytes = NbtFixtures::every_type();
    let (name, root) = NbtReader::read(&bytes, NbtLimits::STRUCTURE).unwrap();

    assert_eq!(name, "root");
    assert_eq!(root.get("byte"), Some(&NbtTag::Byte(-5)));
    assert_eq!(root.get("short"), Some(&NbtTag::Short(-1234)));
    assert_eq!(root.get("int"), Some(&NbtTag::Int(0x0102_0304)));
    assert_eq!(root.get("long"), Some(&NbtTag::Long(-9_000_000_000)));
    assert_eq!(root.get("float"), Some(&NbtTag::Float(1.5)));
    assert_eq!(root.get("double"), Some(&NbtTag::Double(-2.25)));
    assert_eq!(root.get("bytes"), Some(&NbtTag::ByteArray(vec![1, -2, 3])));
    assert_eq!(root.get("string"), Some(&NbtTag::String("hello".to_string())));
    assert_eq!(root.get("list"), Some(&NbtTag::List(vec![NbtTag::Short(1), NbtTag::Short(2)])));
    let Some(NbtTag::Compound(inner)) = root.get("compound") else {
        panic!("compound entry missing");
    };
    assert_eq!(inner.get("k"), Some(&NbtTag::Int(7)));
    assert_eq!(root.get("ints"), Some(&NbtTag::IntArray(vec![10, -20, 30])));
    assert_eq!(root.get("longs"), Some(&NbtTag::LongArray(vec![i64::MIN, 0, i64::MAX])));
    assert_eq!(root.get("missing"), None);
}

#[test]
fn keeps_entry_order() {
    let bytes =
        NbtWriter::compound("", &[("zeta", NbtTag::Byte(1)), ("alpha", NbtTag::Byte(2)), ("mid", NbtTag::Byte(3))])
            .expect("fixture NBT is writable");
    let (_, root) = NbtReader::read(&bytes, NbtLimits::STRUCTURE).unwrap();

    let names: Vec<&str> = root.iter().map(|(name, _)| name).collect();
    assert_eq!(names, vec!["zeta", "alpha", "mid"]);
}

#[test]
fn rejects_truncated_input() {
    let bytes = NbtFixtures::every_type();

    for cut in 0..bytes.len() {
        let result = NbtReader::read(&bytes[..cut], NbtLimits::STRUCTURE);
        assert!(matches!(result, Err(NbtError::Truncated { .. })), "cut at {cut} gave {result:?}");
    }
}

#[test]
fn rejects_unknown_tag_id() {
    // Root compound, empty name, one entry with tag id 13.
    let bytes = [10, 0, 0, 13, 1, 0, b'x', 0, 0];
    let result = NbtReader::read(&bytes, NbtLimits::STRUCTURE);

    assert_eq!(result, Err(NbtError::UnknownTag { id: 13, offset: 3 }));
}

#[test]
fn rejects_depth_past_limit() {
    let limits = NbtLimits { max_depth: 64, ..NbtLimits::STRUCTURE };

    let ok = NbtWriter::compound("", &[("c", NbtFixtures::nested(63))]).expect("fixture NBT is writable");
    assert!(NbtReader::read(&ok, limits).is_ok());

    let too_deep = NbtWriter::compound("", &[("c", NbtFixtures::nested(64))]).expect("fixture NBT is writable");
    assert_eq!(NbtReader::read(&too_deep, limits), Err(NbtError::TooDeep));
}

#[test]
fn rejects_list_length_past_limit() {
    // Root compound, empty name, entry "l" of type list, element type byte, length u32::MAX.
    let mut bytes = vec![10, 0, 0, 9, 1, 0, b'l', 1];
    bytes.extend_from_slice(&u32::MAX.to_le_bytes());

    let result = NbtReader::read(&bytes, NbtLimits::STRUCTURE);

    assert_eq!(result, Err(NbtError::ListTooLong { len: u32::MAX }));
}

#[test]
fn rejects_list_that_claims_more_than_the_input_holds() {
    let mut bytes = vec![10, 0, 0, 9, 1, 0, b'l', 4];
    bytes.extend_from_slice(&1_000_000u32.to_le_bytes());

    let result = NbtReader::read(&bytes, NbtLimits::STRUCTURE);

    assert!(matches!(result, Err(NbtError::Truncated { .. })));
}

#[test]
fn rejects_decoded_size_past_limit() {
    let bytes = NbtWriter::compound("", &[("s", NbtTag::String("x".repeat(100)))]).expect("fixture NBT is writable");
    let limits = NbtLimits { max_total_bytes: 50, ..NbtLimits::STRUCTURE };

    assert_eq!(NbtReader::read(&bytes, limits), Err(NbtError::TooLarge));
}

#[test]
fn rejects_invalid_utf8_string() {
    // Entry "s" of type string whose value is the single byte 0xFF.
    let bytes = [10, 0, 0, 8, 1, 0, b's', 1, 0, 0xFF, 0];
    let result = NbtReader::read(&bytes, NbtLimits::STRUCTURE);

    assert_eq!(result, Err(NbtError::BadUtf8 { offset: 9 }));
}

#[test]
fn rejects_root_that_is_not_a_compound() {
    // Root tag id 9 (list).
    let bytes = [9, 0, 0, 1, 0, 0, 0, 0];
    let result = NbtReader::read(&bytes, NbtLimits::STRUCTURE);

    assert_eq!(result, Err(NbtError::RootNotCompound));
}

#[test]
fn reads_modified_utf8_names() {
    let bytes = NbtWriter::compound("café", &[("漢字é", NbtTag::Byte(1))]).expect("fixture NBT is writable");
    let (name, root) = NbtReader::read(&bytes, NbtLimits::STRUCTURE).unwrap();

    assert_eq!(name, "café");
    assert_eq!(root.get("漢字é"), Some(&NbtTag::Byte(1)));
}

#[test]
fn rejects_large_arrays_past_byte_limit() {
    let limits = NbtLimits { max_total_bytes: 1000, ..NbtLimits::STRUCTURE };

    let ints = NbtWriter::compound("", &[("a", NbtTag::IntArray(vec![0; 1000]))]).expect("fixture NBT is writable");
    assert_eq!(NbtReader::read(&ints, limits), Err(NbtError::TooLarge));

    let longs = NbtWriter::compound("", &[("a", NbtTag::LongArray(vec![0; 1000]))]).expect("fixture NBT is writable");
    assert_eq!(NbtReader::read(&longs, limits), Err(NbtError::TooLarge));
}

#[test]
fn rejects_large_list_past_byte_limit() {
    let limits = NbtLimits { max_total_bytes: 1000, ..NbtLimits::STRUCTURE };
    let items = vec![NbtTag::Byte(1); 200];
    let bytes = NbtWriter::compound("", &[("l", NbtTag::List(items))]).expect("fixture NBT is writable");

    assert_eq!(NbtReader::read(&bytes, limits), Err(NbtError::TooLarge));
}

#[test]
fn rejects_flood_of_compound_entries_past_byte_limit() {
    let limits = NbtLimits { max_total_bytes: 1000, ..NbtLimits::STRUCTURE };
    let names: Vec<String> = (0..300).map(|i| format!("e{i}")).collect();
    let entries: Vec<(&str, NbtTag)> = names.iter().map(|n| (n.as_str(), NbtTag::Byte(1))).collect();
    let bytes = NbtWriter::compound("", &entries).expect("fixture NBT is writable");

    assert_eq!(NbtReader::read(&bytes, limits), Err(NbtError::TooLarge));
    assert!(NbtReader::read(&bytes, NbtLimits::STRUCTURE).is_ok());
}

#[test]
fn reads_int_lists_as_int_list() {
    let bytes = NbtWriter::compound(
        "",
        &[
            ("ints", NbtTag::IntList(vec![1, -2, 3])),
            ("none", NbtTag::IntList(Vec::new())),
            ("shorts", NbtTag::List(vec![NbtTag::Short(4)])),
            ("nested", NbtTag::List(vec![NbtTag::IntList(vec![5, 6])])),
        ],
    )
    .expect("fixture NBT is writable");
    let (_, root) = NbtReader::read(&bytes, NbtLimits::STRUCTURE).unwrap();

    assert_eq!(root.get("ints"), Some(&NbtTag::IntList(vec![1, -2, 3])));
    assert_eq!(root.get("none"), Some(&NbtTag::IntList(Vec::new())));
    assert_eq!(root.get("shorts"), Some(&NbtTag::List(vec![NbtTag::Short(4)])));
    assert_eq!(root.get("nested"), Some(&NbtTag::List(vec![NbtTag::IntList(vec![5, 6])])));
}

#[test]
fn long_int_list_stays_in_the_input_as_a_span() {
    let limits = NbtLimits { max_decoded_int_list: 4, ..NbtLimits::STRUCTURE };
    let bytes = NbtWriter::compound(
        "",
        &[("short", NbtTag::IntList(vec![1, 2, 3, 4])), ("long", NbtTag::IntList(vec![7, -1, 9, 10, 11]))],
    )
    .expect("fixture NBT is writable");
    let (_, root) = NbtReader::read(&bytes, limits).unwrap();

    assert_eq!(root.get("short"), Some(&NbtTag::IntList(vec![1, 2, 3, 4])));
    let Some(NbtTag::IntListSpan { offset, len }) = root.get("long") else {
        panic!("expected a span, got {:?}", root.get("long"));
    };
    assert_eq!(*len, 5);
    let raw: Vec<u8> = [7i32, -1, 9, 10, 11].iter().flat_map(|value| value.to_le_bytes()).collect();
    assert_eq!(&bytes[*offset..*offset + 20], raw.as_slice());
}

#[test]
fn span_is_charged_two_bytes_per_item_and_checked_against_the_input() {
    let limits = NbtLimits { max_decoded_int_list: 4, max_total_bytes: 1000, ..NbtLimits::STRUCTURE };
    // 300 items charge 600 bytes (plus the root's entry slots); 600 items charge 1,200.
    let fits = NbtWriter::compound("", &[("l", NbtTag::IntList(vec![0; 300]))]).expect("fixture NBT is writable");
    let past = NbtWriter::compound("", &[("l", NbtTag::IntList(vec![0; 600]))]).expect("fixture NBT is writable");
    assert!(NbtReader::read(&fits, limits).is_ok());
    assert_eq!(NbtReader::read(&past, limits), Err(NbtError::TooLarge));

    let cut = &fits[..fits.len() - 5];
    assert!(matches!(NbtReader::read(cut, limits), Err(NbtError::Truncated { .. })));
    let too_long = NbtLimits { max_list_len: 299, ..limits };
    assert_eq!(NbtReader::read(&fits, too_long), Err(NbtError::ListTooLong { len: 300 }));
}

#[test]
fn rejects_int_list_past_byte_limit() {
    // 300 ints decode to 1200 bytes.
    let limits = NbtLimits { max_total_bytes: 1000, ..NbtLimits::STRUCTURE };
    let bytes = NbtWriter::compound("", &[("l", NbtTag::IntList(vec![0; 300]))]).expect("fixture NBT is writable");

    assert_eq!(NbtReader::read(&bytes, limits), Err(NbtError::TooLarge));
    assert!(NbtReader::read(&bytes, NbtLimits::STRUCTURE).is_ok());
}

#[test]
fn long_int_array_stays_in_the_input_as_a_span() {
    let limits = NbtLimits { max_decoded_int_list: 4, ..NbtLimits::STRUCTURE };
    let bytes = NbtWriter::compound(
        "",
        &[("short", NbtTag::IntArray(vec![1, 2, 3, 4])), ("long", NbtTag::IntArray(vec![7, -1, 9, 10, 11]))],
    )
    .expect("fixture NBT is writable");
    let (_, root) = NbtReader::read(&bytes, limits).unwrap();

    assert_eq!(root.get("short"), Some(&NbtTag::IntArray(vec![1, 2, 3, 4])));
    let Some(NbtTag::IntArraySpan { offset, len }) = root.get("long") else {
        panic!("expected a span, got {:?}", root.get("long"));
    };
    assert_eq!(*len, 5);
    let raw: Vec<u8> = [7i32, -1, 9, 10, 11].iter().flat_map(|value| value.to_le_bytes()).collect();
    assert_eq!(&bytes[*offset..*offset + 20], raw.as_slice());
}

#[test]
fn int_array_span_is_charged_two_bytes_per_item_and_checked_against_the_input() {
    let limits = NbtLimits { max_decoded_int_list: 4, max_total_bytes: 1000, ..NbtLimits::STRUCTURE };
    // 300 items charge 600 bytes as a span; decoded they would charge 1,200.
    let fits = NbtWriter::compound("", &[("a", NbtTag::IntArray(vec![0; 300]))]).expect("fixture NBT is writable");
    let past = NbtWriter::compound("", &[("a", NbtTag::IntArray(vec![0; 600]))]).expect("fixture NBT is writable");
    assert!(NbtReader::read(&fits, limits).is_ok());
    assert_eq!(NbtReader::read(&past, limits), Err(NbtError::TooLarge));

    let cut = &fits[..fits.len() - 5];
    assert!(matches!(NbtReader::read(cut, limits), Err(NbtError::Truncated { .. })));
    let too_long = NbtLimits { max_list_len: 299, ..limits };
    assert_eq!(NbtReader::read(&fits, too_long), Err(NbtError::ListTooLong { len: 300 }));
}
