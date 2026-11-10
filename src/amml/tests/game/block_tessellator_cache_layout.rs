use crate::support::FakeBuffer;
use alaydriem_stratum::game::BlockTessellatorCacheLayout;

#[test]
fn block_tessellator_cache_layout_is_the_272032_byte_window() {
    assert_eq!(size_of::<BlockTessellatorCacheLayout>(), 272_032);
}

#[test]
fn block_tessellator_cache_layout_reads_the_header_offsets() {
    let mut buffer = FakeBuffer::new(272_032);
    buffer.put_u64(0, 0xA1);
    buffer.put_u32(8, 5);
    buffer.put_u32(12, 6);
    buffer.put_u32(16, 7);
    buffer.put_u64(24, 0xB2);
    buffer.put_u16(32, 0x0F0F);
    buffer.put_u16(32 + 7999 * 2, 0x0E0E);
    buffer.put_u64(16032, 0xC3);
    buffer.put_u64(16032 + 7999 * 8, 0xC4);
    buffer.put_u64(80032, 0xD5);
    buffer.put_u64(144032, 0xE6);
    buffer.put_u64(208032, 0xF7);

    let cache = buffer.view::<BlockTessellatorCacheLayout>();

    assert_eq!(cache.air_block as usize, 0xA1);
    assert_eq!(cache.pos, [5, 6, 7]);
    assert_eq!(cache.region as usize, 0xB2);
    assert_eq!(cache.light_colors[0], 0x0F0F);
    assert_eq!(cache.light_colors[7999], 0x0E0E);
    assert_eq!(cache.blocks[0] as usize, 0xC3);
    assert_eq!(cache.blocks[7999] as usize, 0xC4);
    assert_eq!(cache.extra_blocks[0] as usize, 0xD5);
    assert_eq!(cache.block_graphics[0][0] as usize, 0xE6);
    assert_eq!(cache.block_graphics[1][0] as usize, 0xF7);
}
