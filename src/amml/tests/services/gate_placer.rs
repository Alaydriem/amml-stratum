use alaydriem_stratum::services::GatePlacer;

const FEET: [i32; 3] = [4, 72, 263];
const SIZE: [u32; 3] = [16, 4, 6];

// the structure box from its minimum corner holds no cell of the player's column at the feet or the head
fn holds_player(origin: [i32; 3]) -> bool {
    let inside = |axis: usize, value: i32| value >= origin[axis] && value < origin[axis] + SIZE[axis] as i32;
    inside(0, FEET[0]) && inside(2, FEET[2]) && (inside(1, FEET[1]) || inside(1, FEET[1] + 1))
}

#[test]
fn facing_minus_x_the_structure_ends_two_blocks_ahead_and_is_centred_across() {
    let origin = GatePlacer::origin_ahead(FEET, 90.0, SIZE);

    assert_eq!(origin, [4 - 2 - 15, 72, 263 - 3]);
    assert!(!holds_player(origin));
}

#[test]
fn facing_plus_z_the_structure_starts_two_blocks_ahead() {
    let origin = GatePlacer::origin_ahead(FEET, 0.0, SIZE);

    assert_eq!(origin, [4 - 8, 72, 263 + 2]);
    assert!(!holds_player(origin));
}

#[test]
fn every_facing_keeps_the_player_outside() {
    for yaw in (-180..180).step_by(15) {
        assert!(!holds_player(GatePlacer::origin_ahead(FEET, yaw as f32, SIZE)), "yaw {yaw}");
    }
}
