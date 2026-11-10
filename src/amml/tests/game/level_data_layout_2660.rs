use crate::support::FakeBuffer;
use alaydriem_stratum::game::LevelDataLayout2660;

#[test]
fn level_data_layout_2660_reads_the_level_name_8_bytes_later() {
    let mut buffer = FakeBuffer::new(0x2D0);
    // MSVC std::string with SSO: the text in the 16-byte buffer, the size at +10h, capacity 15 at +18h
    for (index, byte) in b"Creative".iter().enumerate() {
        buffer.put_u8(0x2B0 + index, *byte);
    }
    buffer.put_u64(0x2B0 + 0x10, 8);
    buffer.put_u64(0x2B0 + 0x18, 15);

    let level_data = buffer.view::<LevelDataLayout2660>();

    assert_eq!(level_data.level_name.to_str().unwrap(), "Creative");
}
