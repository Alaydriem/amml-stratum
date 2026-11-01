use alaydriem_stratum::game::GameModeLayout;

use crate::support::FakeBuffer;

#[test]
fn game_mode_layout_reads_the_player_and_the_build_delay_at_the_header_offsets() {
    let mut buffer = FakeBuffer::new(0xB0);
    // the 26.51 constructor: the player at 8, then the three delays 250, 200, 200 at 0xA0, 0xA8 and 0xB0 (the last
    // past the prefix)
    buffer.put_u64(0x08, 0x5000);
    buffer.put_u64(0xA0, 250);
    buffer.put_u64(0xA8, 200);

    let game_mode = buffer.view::<GameModeLayout>();

    assert_eq!(game_mode.player as usize, 0x5000);
    assert_eq!(game_mode.building_tick_delay, 200);
}
