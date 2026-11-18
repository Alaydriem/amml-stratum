use std::ffi::c_void;

use alaydriem_stratum::game::{BlockName, ItemName};

// the offset the fake block type holds its name at
const NAME_AT: u64 = 0xE0;

// a fake block type: the hash qword at the offset, then the 32-byte string with its text inline
fn fake_block_type(name: &str, hash: u64) -> Vec<u64> {
    let mut words = vec![0u64; 0x40];
    let at = NAME_AT as usize / 8;
    words[at] = hash;
    // SAFETY: the buffer holds the string's 32 bytes after the hash
    unsafe {
        let text = words.as_mut_ptr().add(at + 1).cast::<u8>();
        std::ptr::copy_nonoverlapping(name.as_ptr(), text, name.len());
    }
    words[at + 3] = name.len() as u64;
    words[at + 4] = 15;
    words
}

#[allow(unsafe_code)]
fn name_of(block_type: &[u64]) -> Option<String> {
    // SAFETY: a fake laid out as the prefix the reader reads
    unsafe { BlockName::of(block_type.as_ptr().cast::<c_void>(), NAME_AT) }
}

#[test]
fn a_name_with_the_right_hash_is_read() {
    let block_type = fake_block_type("minecraft:dirt", ItemName::hash(b"minecraft:dirt"));

    assert_eq!(name_of(&block_type).as_deref(), Some("minecraft:dirt"));
}

#[test]
fn a_name_with_another_hash_is_not_given() {
    let block_type = fake_block_type("minecraft:dirt", 0x1234);

    assert_eq!(name_of(&block_type), None);
}

#[test]
#[allow(unsafe_code)]
fn a_null_block_type_has_no_name() {
    // SAFETY: a null type is allowed
    assert_eq!(unsafe { BlockName::of(std::ptr::null(), NAME_AT) }, None);
}
