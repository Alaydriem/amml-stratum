use crate::support::FakeBuffer;
use alaydriem_stratum::game::BlockTessellatorLayout;

#[test]
fn block_tessellator_layout_is_the_293344_byte_object() {
    assert_eq!(size_of::<BlockTessellatorLayout>(), 293_344);
}

#[test]
fn block_tessellator_layout_reads_the_flags_layer_and_cache() {
    let mut buffer = FakeBuffer::new(293_344);
    buffer.put_u8(1, 1);
    buffer.put_u64(8, 0xABCD);
    buffer.put_u32(128, 2);
    buffer.put_u8(232, 1);
    buffer.put_u8(241, 1);
    // the cache position at mBlockCache + 8, the cache region at mBlockCache + 24
    buffer.put_u32(1712 + 8, 0xFFFF_FFFE);
    buffer.put_u64(1712 + 24, 0x1234);

    let tessellator = buffer.view::<BlockTessellatorLayout>();

    assert!(tessellator.use_normals);
    assert_eq!(tessellator.region as usize, 0xABCD);
    assert_eq!(tessellator.rendering_layer, 2);
    assert!(tessellator.apply_ambient_occlusion);
    assert!(tessellator.use_cached_calculation);
    assert_eq!(tessellator.block_cache.pos[0], -2);
    assert_eq!(tessellator.block_cache.region as usize, 0x1234);
}
