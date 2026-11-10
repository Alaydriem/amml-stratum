use stratum_core::config::PreviewConfig;

#[test]
fn each_bound_is_held_between_its_floor_and_its_ceiling() {
    let low = PreviewConfig { texture_cache: 0, max_quads: 0 };
    assert_eq!(
        (low.texture_cache(), low.max_quads()),
        (PreviewConfig::MIN_TEXTURE_CACHE, PreviewConfig::MIN_MAX_QUADS)
    );
    let high = PreviewConfig { texture_cache: usize::MAX, max_quads: usize::MAX };
    assert_eq!(
        (high.texture_cache(), high.max_quads()),
        (PreviewConfig::MAX_TEXTURE_CACHE, PreviewConfig::MAX_MAX_QUADS)
    );
    let inside = PreviewConfig { texture_cache: 99, max_quads: 5000 };
    assert_eq!((inside.texture_cache(), inside.max_quads()), (99, 5000));
}
