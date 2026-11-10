use std::ffi::c_void;

use alaydriem_stratum::game::ItemName;

// Item::mFullName in the 26.51 header: the vptr, then mItemParseVersion .. mNamespace; HashedString is the hash, the
// std::string (16 bytes of text or heap pointer, the size, the capacity), then mLastMatch
const FULL_NAME: usize = 0x120;
const TEXT: usize = FULL_NAME + 0x8;

// HashedString::computeHash as the 26.51 header writes it: `hash = s ^ (0x100000001B3 * hash)` over each `char`,
// with 0 for an empty string
fn header_hash(text: &str) -> u64 {
    if text.is_empty() {
        return 0;
    }
    text.bytes().fold(0xCBF2_9CE4_8422_2325u64, |hash, byte| {
        (byte as i8 as i64 as u64) ^ 0x0000_0100_0000_01B3u64.wrapping_mul(hash)
    })
}

// a fake Item of 0x180 bytes with the name at the header offsets; a name over 15 bytes lives in `heap`
fn fake_item(name: &str, hash: u64, heap: &mut Vec<u8>) -> Vec<u64> {
    let mut item = vec![0u64; 0x180 / 8];
    let bytes = item.as_mut_ptr().cast::<u8>();
    // SAFETY: every write stays inside the 0x180-byte fake
    #[allow(unsafe_code)]
    unsafe {
        bytes.add(FULL_NAME).cast::<u64>().write(hash);
        if name.len() <= 15 {
            std::ptr::copy_nonoverlapping(name.as_ptr(), bytes.add(TEXT), name.len());
            bytes.add(TEXT + 0x18).cast::<usize>().write(15);
        } else {
            heap.clear();
            heap.extend_from_slice(name.as_bytes());
            heap.push(0);
            bytes.add(TEXT).cast::<*const u8>().write(heap.as_ptr());
            bytes.add(TEXT + 0x18).cast::<usize>().write(name.len());
        }
        bytes.add(TEXT + 0x10).cast::<usize>().write(name.len());
    }
    item
}

#[allow(unsafe_code)]
fn name_of(item: &[u64]) -> Option<String> {
    // SAFETY: a fake laid out as the Item prefix the reader reads
    unsafe { ItemName::of(item.as_ptr().cast::<c_void>()) }
}

#[test]
fn a_short_name_inline_in_the_item_is_read_with_the_header_hash() {
    let mut heap = Vec::new();
    let item = fake_item("minecraft:dirt", header_hash("minecraft:dirt"), &mut heap);

    assert_eq!(name_of(&item).as_deref(), Some("minecraft:dirt"));
}

#[test]
fn a_long_name_in_its_heap_block_is_read_with_the_header_hash() {
    let mut heap = Vec::new();
    let item = fake_item("minecraft:oak_stairs", header_hash("minecraft:oak_stairs"), &mut heap);

    assert_eq!(name_of(&item).as_deref(), Some("minecraft:oak_stairs"));
}

#[test]
fn a_name_whose_hash_does_not_match_gives_none() {
    let mut heap = Vec::new();
    let item = fake_item("minecraft:stone", header_hash("minecraft:dirt"), &mut heap);

    assert_eq!(name_of(&item), None);
}

#[test]
fn a_null_item_gives_none() {
    // SAFETY: a null item is allowed
    #[allow(unsafe_code)]
    let name = unsafe { ItemName::of(std::ptr::null()) };
    assert_eq!(name, None);
}
