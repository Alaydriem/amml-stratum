use alaydriem_stratum::game::DbStorage;

use crate::support::FakeBuffer;

// A storage whose mFullPath holds `path`, in the heap form of an MSVC std::string: the text pointer, the size at +10h
// and a capacity of at least 16 at +18h. `text` must outlive the reads.
fn storage(text: &[u8]) -> FakeBuffer {
    let mut buffer = FakeBuffer::new(0xB8);
    buffer.put_u64(0x98, text.as_ptr() as u64);
    buffer.put_u64(0x98 + 0x10, text.len() as u64);
    buffer.put_u64(0x98 + 0x18, (text.len() as u64).max(16) | 15);
    buffer
}

#[test]
fn the_folder_name_is_the_last_part_of_the_world_path() {
    let path = br"C:\Users\Steve\AppData\Roaming\Minecraft Bedrock\minecraftWorlds\My World";
    let mut buffer = storage(path);

    // SAFETY: the buffer is laid out as the head of a constructed DBStorage
    let name = unsafe { DbStorage::folder_name(buffer.as_mut_ptr()) };

    assert_eq!(name.as_deref(), Some("My World"));
}

#[test]
fn a_trailing_separator_still_names_the_folder() {
    let path = b"C:/games/minecraftWorlds/AbCdEfGhIjk=/";
    let mut buffer = storage(path);

    // SAFETY: as above
    let name = unsafe { DbStorage::folder_name(buffer.as_mut_ptr()) };

    assert_eq!(name.as_deref(), Some("AbCdEfGhIjk="));
}

#[test]
fn an_empty_path_or_a_null_storage_names_nothing() {
    let mut buffer = FakeBuffer::new(0xB8);
    buffer.put_u64(0x98 + 0x18, 15);

    // SAFETY: an empty SSO string in a DBStorage head, then a null pointer
    let (empty, null) =
        unsafe { (DbStorage::folder_name(buffer.as_mut_ptr()), DbStorage::folder_name(std::ptr::null())) };

    assert_eq!((empty, null), (None, None));
}
