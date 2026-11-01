use crate::support::FakeBuffer;
use alaydriem_stratum::game::ActorConstantsLayout;

#[test]
fn actor_constants_layout_reads_the_eleven_constant_pointers() {
    let mut buffer = FakeBuffer::new(104);
    buffer.put_u64(0x10, 0x101);
    buffer.put_u64(0x38, 0x106);
    buffer.put_u64(0x60, 0x111);

    let constants = buffer.view::<ActorConstantsLayout>();

    assert_eq!(size_of::<ActorConstantsLayout>(), 104);
    assert_eq!(constants.overlay_color as usize, 0x101);
    assert_eq!(constants.glint_color as usize, 0x106);
    assert_eq!(constants.uv_scale as usize, 0x111);
}
