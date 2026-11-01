use alaydriem_stratum::game::{GameMode, PlayerInventorySpanLayout};
use alaydriem_stratum::types::UseCalls;

use crate::support::{FakeBuffer, FakeUse};

const ANCHOR: usize = 0x5B8;

#[test]
fn the_span_from_the_inventory_anchor_reads_the_game_mode_at_the_header_offset() {
    let mut buffer = FakeBuffer::new(0x4F0);
    buffer.put_u64(0x000, 0x6000);
    buffer.put_u64(0x4E8, 0x7000);

    let span = buffer.view::<PlayerInventorySpanLayout>();

    assert_eq!(span.inventory as usize, 0x6000);
    assert_eq!(span.game_mode as usize, 0x7000);
}

#[test]
#[allow(unsafe_code)]
fn a_player_laid_out_as_the_header_gives_its_game_mode_and_build_delay() {
    let rig = FakeUse::new();
    let mut game_mode = FakeBuffer::new(0xB0);
    game_mode.put_u64(0x08, 0x5000);
    game_mode.put_u64(0xA8, 150);
    let mut player = FakeBuffer::new(0xAA8);
    // Player::mInventory at the 0x5B8 anchor and Player::mGameMode at 0xAA0, from the 26.51 header sum
    player.put_u64(ANCHOR, 0x6000);
    player.put_u64(0xAA0, game_mode.as_mut_ptr() as u64);
    let players = alaydriem_stratum::types::PlayerCalls { player_inventory: ANCHOR as u64, ..rig.players };
    let calls: UseCalls = rig.calls;

    // SAFETY: a fake player laid out at the header offsets, and its fake game mode
    let found = unsafe { GameMode::of_player(player.as_mut_ptr(), &players, &calls) }.expect("the player has one");

    assert_eq!(found.as_ptr(), game_mode.as_mut_ptr());
    assert_eq!((found.player(), found.building_tick_delay_ms()), (0x5000, 150));
}
