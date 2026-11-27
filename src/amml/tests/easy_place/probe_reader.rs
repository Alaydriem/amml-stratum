use alaydriem_stratum::easy_place::ProbeReader;
use amml_api::BlockPos;
use stratum_core::structs::block::Face;
use stratum_core::structs::easy_place::{GameMode, SwapAnswer, UseProbe};

fn probe(eye: [f64; 3], dir: [f64; 3], reach: f64) -> UseProbe {
    UseProbe {
        time_seconds: 0.0,
        pressed: false,
        held: true,
        eye,
        dir,
        reach,
        game_mode: GameMode::Survival,
        hit: None,
        body: None,
        swap_answer: SwapAnswer::None,
    }
}

#[test]
fn the_walk_stops_at_the_first_block_that_cannot_be_built_over() {
    let mut probe = probe([0.5, 70.5, 0.5], [0.0, -1.0, 0.0], 10.0);

    // grass at 68 is walked through, stone at 66 is hit
    let hit = ProbeReader::walk(|pos: BlockPos| pos.y != 66, &mut probe);

    assert!(hit);
    let hit = probe.hit.unwrap();
    assert_eq!(hit.cell, [0, 66, 0]);
    assert_eq!(hit.face, Face::Up);
    assert!((hit.distance - 3.5).abs() < 1e-9);
}

#[test]
fn nothing_past_the_reach_is_hit() {
    let mut probe = probe([0.5, 70.5, 0.5], [0.0, -1.0, 0.0], 2.0);

    let hit = ProbeReader::walk(|pos: BlockPos| pos.y != 66, &mut probe);

    assert!(!hit);
    assert!(probe.hit.is_none());
}

#[test]
fn a_ray_along_x_enters_through_the_west_face() {
    let mut probe = probe([0.5, 64.5, 0.5], [1.0, 0.0, 0.0], 6.0);

    ProbeReader::walk(|pos: BlockPos| pos.x != 3, &mut probe);

    assert_eq!(probe.hit.map(|hit| (hit.cell, hit.face)), Some(([3, 64, 0], Face::West)));
}

#[test]
fn the_body_is_the_cells_the_box_overlaps_and_an_edge_does_not_reach_the_next_cell() {
    let body = ProbeReader::body([9.7, 64.0, -20.3], [10.3, 65.8, -19.7]);

    assert_eq!(body, Some(([9, 64, -21], [10, 65, -20])));
    assert_eq!(ProbeReader::body([1.0, 1.0, 1.0], [1.0, 2.0, 2.0]), None);
}
