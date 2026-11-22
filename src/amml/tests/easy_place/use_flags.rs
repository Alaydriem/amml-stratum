use alaydriem_stratum::easy_place::UseFlags;

fn holding_and_blocking() -> UseFlags {
    let flags = UseFlags::new();
    flags.set_holding(true);
    flags.set_blocking(true);
    flags
}

#[test]
fn a_blocking_hold_skips_the_game_build_of_the_local_player() {
    assert!(holding_and_blocking().blocks_game_build(true, false));
}

#[test]
fn the_mod_own_build_and_another_player_build_always_go_through() {
    let flags = holding_and_blocking();

    assert!(!flags.blocks_game_build(true, true), "the mod's own build_at");
    assert!(!flags.blocks_game_build(false, false), "the server thread, or another player");
}

#[test]
fn without_a_blocking_hold_nothing_is_skipped() {
    let flags = UseFlags::new();
    assert!(!flags.blocks_game_build(true, false));
    flags.set_holding(true);
    assert!(!flags.blocks_game_build(true, false));
}
