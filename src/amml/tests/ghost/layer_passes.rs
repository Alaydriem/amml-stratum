use alaydriem_stratum::ghost::LayerPasses;

fn layers(passes: LayerPasses) -> Vec<i32> {
    passes.layers().collect()
}

#[test]
fn a_block_with_no_extra_layers_is_one_pass_on_its_main_layer() {
    assert_eq!(layers(LayerPasses::new(5, 0)), vec![5]);
    assert_eq!(LayerPasses::new(5, 0).count(), 1);
}

#[test]
fn honey_draws_on_blend_then_its_extra_opaque_pass() {
    assert_eq!(layers(LayerPasses::new(3, 0x20)), vec![3, 5]);
    assert_eq!(LayerPasses::new(3, 0x20).count(), 2);
}

#[test]
fn an_extra_bit_equal_to_the_main_layer_is_not_a_second_pass() {
    assert_eq!(layers(LayerPasses::new(3, 0x08)), vec![3]);
}

#[test]
fn bits_past_the_layer_count_are_ignored() {
    assert_eq!(layers(LayerPasses::new(8, (1 << 22) | (1 << 31))), vec![8]);
    assert_eq!(LayerPasses::new(8, (1 << 22) | (1 << 31)).count(), 1);
}
