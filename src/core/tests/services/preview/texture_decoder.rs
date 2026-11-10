use stratum_core::services::preview::TextureDecoder;
use stratum_core::structs::preview::TextureTint;

use crate::support::PreviewFixture;

const RED: [u8; 4] = [255, 0, 0, 255];
const BLUE: [u8; 4] = [0, 0, 255, 255];

#[test]
fn tga_rows_stored_bottom_first_come_out_top_first() {
    let bytes = PreviewFixture::tga(1, &[vec![RED], vec![BLUE]]);
    let (width, height, rgba) = TextureDecoder::tga(&bytes).unwrap();
    assert_eq!((width, height), (1, 2));
    assert_eq!(&rgba[..4], &RED);
    assert_eq!(&rgba[4..], &BLUE);
}

#[test]
fn run_length_tga_repeats_a_texel() {
    let mut bytes = vec![0u8; 18];
    bytes[2] = 10;
    bytes[12] = 3;
    bytes[14] = 1;
    bytes[16] = 24;
    bytes[17] = 0x20;
    // one run packet of three texels, stored blue, green, red
    bytes.extend_from_slice(&[0x82, 30, 20, 10]);
    let (_, _, rgba) = TextureDecoder::tga(&bytes).unwrap();
    assert_eq!(rgba, [10, 20, 30, 255].repeat(3));
}

#[test]
fn an_animated_strip_keeps_its_first_frame() {
    let rows = vec![vec![RED], vec![BLUE], vec![BLUE]];
    let texture = TextureDecoder::decode(&PreviewFixture::tga(1, &rows), true, TextureTint::None).unwrap();
    assert_eq!((texture.width(), texture.height()), (1, 1));
    assert_eq!(texture.rgba(), &RED);
}

#[test]
fn overlay_tint_colours_only_marked_texels_and_makes_all_opaque() {
    let marked = [200, 200, 200, 255];
    let unmarked = [100, 50, 25, 0];
    let mut rgba = [marked, unmarked].concat();
    TextureTint::Overlay(TextureTint::PLAINS_GRASS).apply(&mut rgba);
    let [r, g, b] = TextureTint::PLAINS_GRASS.map(|tint| ((200 * u16::from(tint) + 127) / 255) as u8);
    assert_eq!(&rgba[..4], &[r, g, b, 255]);
    assert_eq!(&rgba[4..], &[100, 50, 25, 255]);
}

#[test]
fn leaves_take_the_foliage_tint_and_their_opaque_variant_does_not() {
    assert_eq!(
        TextureTint::for_path("textures/blocks/leaves_oak", false),
        TextureTint::Multiply(TextureTint::PLAINS_FOLIAGE)
    );
    assert_eq!(TextureTint::for_path("textures/blocks/leaves_oak_opaque", false), TextureTint::None);
    assert_eq!(TextureTint::for_path("textures/blocks/stone", false), TextureTint::None);
}
