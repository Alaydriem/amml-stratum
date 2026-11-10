use stratum_core::structs::preview::TextureTint;

#[test]
fn each_texture_path_gets_its_plains_tint() {
    let grass = TextureTint::Overlay(TextureTint::PLAINS_GRASS);
    assert_eq!(TextureTint::for_path("textures/blocks/grass_side", true), grass);
    let spruce = TextureTint::Multiply(TextureTint::SPRUCE_FOLIAGE);
    assert_eq!(TextureTint::for_path("textures/blocks/leaves_spruce", false), spruce);
    let foliage = TextureTint::Multiply(TextureTint::PLAINS_FOLIAGE);
    assert_eq!(TextureTint::for_path("textures/blocks/leaves_oak", false), foliage);
    let tall = TextureTint::Multiply(TextureTint::PLAINS_GRASS);
    assert_eq!(TextureTint::for_path("textures/blocks/double_plant_grass_top", false), tall);
    let water = TextureTint::Liquid(TextureTint::PLAINS_WATER);
    assert_eq!(TextureTint::for_path("textures/blocks/water_still_grey", false), water);
    assert_eq!(TextureTint::for_path("textures/blocks/stone", false), TextureTint::None);
}

#[test]
fn opaque_and_carried_variants_are_not_tinted_and_an_overlay_wins_over_the_name() {
    assert_eq!(TextureTint::for_path("textures/blocks/leaves_oak_opaque", false), TextureTint::None);
    assert_eq!(TextureTint::for_path("textures/blocks/leaves_oak_carried", false), TextureTint::None);
    let grass = TextureTint::Overlay(TextureTint::PLAINS_GRASS);
    assert_eq!(TextureTint::for_path("textures/blocks/leaves_oak_opaque", true), grass);
}

#[test]
fn multiply_scales_each_colour_channel_and_keeps_alpha() {
    let mut texels = [255, 128, 0, 77, 10, 20, 30, 0];
    TextureTint::Multiply([128, 255, 64]).apply(&mut texels);
    // channel * tint / 255, rounded
    assert_eq!(texels, [128, 128, 0, 77, 5, 20, 8, 0]);
}

#[test]
fn liquid_makes_every_texel_opaque_and_none_changes_nothing() {
    let mut water = [200, 200, 200, 90];
    TextureTint::Liquid([255, 0, 255]).apply(&mut water);
    assert_eq!(water, [200, 0, 200, 255]);
    let mut stone = [1, 2, 3, 4];
    TextureTint::None.apply(&mut stone);
    assert_eq!(stone, [1, 2, 3, 4]);
}

#[test]
fn overlay_tints_only_the_marked_texels_and_makes_all_opaque() {
    let mut texels = [100, 100, 100, 0, 255, 255, 255, 1];
    TextureTint::Overlay([255, 0, 0]).apply(&mut texels);
    assert_eq!(texels, [100, 100, 100, 255, 255, 0, 0, 255]);
}
