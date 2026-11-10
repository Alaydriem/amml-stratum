use alaydriem_stratum::game::{TexturePtrLayout, TextureVariant};

#[test]
fn texture_variant_is_40_bytes_with_the_index_after_the_texture() {
    assert_eq!(size_of::<TextureVariant>(), 40);
    let variant = TextureVariant::new(TexturePtrLayout::VARIANT_INDEX, TexturePtrLayout::EMPTY);
    // SAFETY: TextureVariant is 40 bytes; padding is read as MaybeUninit.
    let bytes: [std::mem::MaybeUninit<u8>; 40] = unsafe { std::mem::transmute(variant) };
    // SAFETY: byte 32 is the index field, written by TextureVariant::new.
    assert_eq!(unsafe { bytes[32].assume_init() }, 1);
}
