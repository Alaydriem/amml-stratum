use stratum_core::services::LayoutSolver;
use stratum_core::structs::layout::LayoutIssue;
use stratum_core::structs::library::StructureId;
use stratum_core::structs::structure::StructureSize;

use crate::support::PieceFixture;

#[test]
fn two_pieces_64_apart_line_up() {
    let a = PieceFixture::at(1, [-128, 60, 0], [64, 40, 64]);
    let b = PieceFixture::at(2, [-64, 60, 0], [64, 40, 64]);
    let layout = LayoutSolver::from_origins(&[&a, &b]).unwrap();
    assert_eq!(PieceFixture::offsets(&layout.pieces), vec![(1, [0, 0, 0]), (2, [64, 0, 0])]);
    assert_eq!(layout.size, StructureSize::new(128, 40, 64));
}

#[test]
fn negative_and_mixed_origins() {
    let a = PieceFixture::at(1, [-64, 10, -64], [64, 20, 64]);
    let b = PieceFixture::at(2, [0, 10, -64], [64, 20, 64]);
    let c = PieceFixture::at(3, [-64, 10, 0], [64, 20, 64]);
    let d = PieceFixture::at(4, [0, 10, 0], [64, 20, 64]);
    let layout = LayoutSolver::from_origins(&[&d, &a, &c, &b]).unwrap();
    assert_eq!(
        PieceFixture::offsets(&layout.pieces),
        vec![(4, [64, 0, 64]), (1, [0, 0, 0]), (3, [0, 0, 64]), (2, [64, 0, 0])]
    );
    assert_eq!(layout.size, StructureSize::new(128, 20, 128));
}

#[test]
fn pieces_at_different_heights_keep_their_y() {
    let a = PieceFixture::at(1, [0, 60, 0], [16, 20, 16]);
    let b = PieceFixture::at(2, [16, 80, 0], [16, 20, 16]);
    let layout = LayoutSolver::from_origins(&[&a, &b]).unwrap();
    assert_eq!(PieceFixture::offsets(&layout.pieces), vec![(1, [0, 0, 0]), (2, [16, 20, 0])]);
    assert_eq!(layout.size, StructureSize::new(32, 40, 16));
}

#[test]
fn missing_origin_is_reported() {
    let a = PieceFixture::at(1, [0, 0, 0], [16, 16, 16]);
    let b = PieceFixture::without_origin(2, [16, 16, 16]);
    let c = PieceFixture::without_origin(3, [16, 16, 16]);
    assert_eq!(
        LayoutSolver::from_origins(&[&a, &b, &c]),
        Err(LayoutIssue::MissingOrigin(vec![StructureId(2), StructureId(3)]))
    );
}

#[test]
fn overlap_is_reported() {
    let a = PieceFixture::at(1, [0, 0, 0], [16, 16, 16]);
    let b = PieceFixture::at(2, [15, 0, 0], [16, 16, 16]);
    let touching = PieceFixture::at(3, [16, 0, 16], [16, 16, 16]);
    assert_eq!(
        LayoutSolver::from_origins(&[&a, &touching, &b]),
        Err(LayoutIssue::Overlap(StructureId(1), StructureId(2)))
    );
    assert!(LayoutSolver::from_origins(&[&a, &touching]).is_ok());
}

#[test]
fn extreme_origins_do_not_panic() {
    let low = PieceFixture::at(1, [i32::MIN, 0, 0], [16, 16, 16]);
    let high = PieceFixture::at(2, [i32::MAX - 16, 0, 0], [16, 16, 16]);
    assert_eq!(LayoutSolver::from_origins(&[&low, &high]), Err(LayoutIssue::TooLarge));
    let near = PieceFixture::at(3, [i32::MIN + 16, 0, 0], [16, 16, 16]);
    let layout = LayoutSolver::from_origins(&[&low, &near]).unwrap();
    assert_eq!(PieceFixture::offsets(&layout.pieces), vec![(1, [0, 0, 0]), (3, [16, 0, 0])]);
}

#[test]
fn grid_places_edge_to_edge_with_uneven_sizes() {
    let a = PieceFixture::in_cell(1, 0, 0, [10, 5, 8]);
    let b = PieceFixture::in_cell(2, 1, 0, [20, 7, 8]);
    let c = PieceFixture::in_cell(3, 0, 1, [12, 5, 30]);
    let d = PieceFixture::in_cell(4, 1, 1, [4, 9, 6]);
    let layout = LayoutSolver::from_grid(&[&a, &b, &c, &d]).unwrap();
    // column 0 is 12 wide (piece c), row 0 is 8 deep
    assert_eq!(
        PieceFixture::offsets(&layout.pieces),
        vec![(1, [0, 0, 0]), (2, [12, 0, 0]), (3, [0, 0, 8]), (4, [12, 0, 8])]
    );
    assert_eq!(layout.size, StructureSize::new(32, 9, 38));
}

#[test]
fn grid_needs_every_cell() {
    let a = PieceFixture::in_cell(1, 0, 0, [10, 5, 8]);
    let b = PieceFixture::without_origin(2, [10, 5, 8]);
    assert_eq!(LayoutSolver::from_grid(&[&a, &b]), Err(LayoutIssue::GridIncomplete(vec![StructureId(2)])));
}

#[test]
fn grid_wins_over_origins_when_complete() {
    let mut a = PieceFixture::in_cell(1, 0, 0, [10, 5, 8]);
    let mut b = PieceFixture::in_cell(2, 1, 0, [10, 5, 8]);
    a.world_origin = Some([500, 0, 500]);
    b.world_origin = Some([0, 0, 0]);
    let layout = LayoutSolver::solve(&[&a, &b]).unwrap();
    assert_eq!(PieceFixture::offsets(&layout.pieces), vec![(1, [0, 0, 0]), (2, [10, 0, 0])]);
}

#[test]
fn solve_falls_back_to_origins_when_a_cell_is_missing() {
    let a = PieceFixture::at(1, [0, 0, 0], [16, 16, 16]);
    let mut b = PieceFixture::at(2, [16, 0, 0], [16, 16, 16]);
    b.grid_cell = Some(stratum_core::structs::layout::GridCell::new(0, 0));
    let layout = LayoutSolver::solve(&[&a, &b]).unwrap();
    assert_eq!(PieceFixture::offsets(&layout.pieces), vec![(1, [0, 0, 0]), (2, [16, 0, 0])]);
}

#[test]
fn two_pieces_in_one_cell_overlap() {
    let a = PieceFixture::in_cell(1, 0, 0, [10, 5, 8]);
    let b = PieceFixture::in_cell(2, 0, 0, [10, 5, 8]);
    assert_eq!(LayoutSolver::from_grid(&[&a, &b]), Err(LayoutIssue::Overlap(StructureId(1), StructureId(2))));
}

#[test]
fn no_pieces_is_an_issue() {
    assert_eq!(LayoutSolver::from_origins(&[]), Err(LayoutIssue::NoPieces));
    assert_eq!(LayoutSolver::from_grid(&[]), Err(LayoutIssue::NoPieces));
    assert_eq!(LayoutSolver::solve(&[]), Err(LayoutIssue::NoPieces));
}
