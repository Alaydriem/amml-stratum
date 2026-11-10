use std::sync::atomic::Ordering;

use alaydriem_stratum::game::OwnedMeshData;

use crate::support::FakeGhost;
use crate::support::fake_ghost::{MESH_DATA_CLEAR_CALLS, MESH_DATA_CLEARED_ZEROED};

#[test]
fn a_new_mesh_data_is_zeroed_and_then_cleared_by_the_game() {
    let data = OwnedMeshData::new(&FakeGhost::calls());

    assert_eq!(MESH_DATA_CLEAR_CALLS.load(Ordering::SeqCst), 1);
    assert!(MESH_DATA_CLEARED_ZEROED.load(Ordering::SeqCst));
    assert_eq!(data.layout().positions.len(), 0);
}
