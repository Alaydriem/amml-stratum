use crate::support::FakeBuffer;
use alaydriem_stratum::game::LevelDataLayout2651;

#[test]
fn level_data_layout_2651_reads_the_level_name_in_place() {
    let mut buffer = FakeBuffer::new(0x2C8);
    // MSVC std::string with SSO: the text in the 16-byte buffer, the size at +10h, capacity 15 at +18h
    for (index, byte) in b"Survival".iter().enumerate() {
        buffer.put_u8(0x2A8 + index, *byte);
    }
    buffer.put_u64(0x2A8 + 0x10, 8);
    buffer.put_u64(0x2A8 + 0x18, 15);

    let level_data = buffer.view::<LevelDataLayout2651>();

    assert_eq!(level_data.level_name.to_str().unwrap(), "Survival");
}
