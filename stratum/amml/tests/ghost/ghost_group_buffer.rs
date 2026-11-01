use alaydriem_stratum::game::{GameVec, OwnedMeshData};
use alaydriem_stratum::ghost::GhostGroupBuffer;
use alaydriem_stratum::types::GroupStep;
use stratum_core::structs::ghost::GhostJobMode;

use crate::support::fake_ghost::{HEAP_FREED, HEAP_LIVE, HEAP_NEW_CALLS, MESH_DATA_DTOR_CALLS};
use crate::support::{FakeGhost, GhostRig};

// a mesh data on the fake heap holding `count` vertices: positions and colours
#[allow(unsafe_code)]
fn part(count: usize) -> OwnedMeshData {
    let mut data = OwnedMeshData::new(&FakeGhost::calls());
    let heap = FakeGhost::heap();
    let positions: Vec<[f32; 3]> = (0..count).map(|index| [index as f32, 0.0, 0.0]).collect();
    let colors = vec![0xFF00_00FFu32; count];
    // SAFETY: the vectors of a new mesh data grow on the fake heap, which the fake dtor frees them to
    unsafe {
        GameVec::extend(&heap, &mut data.layout_mut().positions, &positions).unwrap();
        GameVec::extend(&heap, &mut data.layout_mut().colors, &colors).unwrap();
    }
    data
}

fn group() -> GhostGroupBuffer {
    let mut group = GhostGroupBuffer::new(&FakeGhost::calls());
    group.resize(GhostRig::SLOTS as usize);
    group
}

#[test]
#[allow(unsafe_code)]
fn growth_keeps_the_pointers_in_order_uses_the_game_heap_and_frees_each_old_block_once() {
    let heap = FakeGhost::heap();
    let mut data = OwnedMeshData::new(&FakeGhost::calls());
    let allocations_before = FakeGhost::count(&HEAP_NEW_CALLS);
    let mut olds = Vec::new();

    for round in 0..4u32 {
        let old = data.layout().indices.raw_parts()[0] as usize;
        let items: Vec<u32> = (0..10).map(|index| round * 10 + index).collect();
        // SAFETY: the vector grows on the fake heap it was made on
        unsafe { GameVec::extend(&heap, &mut data.layout_mut().indices, &items) }.unwrap();
        let [first, last, end] = data.layout().indices.raw_parts();
        assert!(first <= last && last <= end, "begin <= end <= cap after round {round}");
        if old != 0 && old != first as usize {
            olds.push(old);
        }
    }

    assert_eq!(data.layout().indices.as_slice(), (0..40).collect::<Vec<u32>>().as_slice());
    let allocated = FakeGhost::count(&HEAP_NEW_CALLS) - allocations_before;
    assert_eq!(allocated as usize, olds.len() + 1, "every block came from the game heap");
    let freed = HEAP_FREED.lock().unwrap().clone();
    for old in olds {
        assert_eq!(freed.iter().filter(|block| **block == old).count(), 1, "an old block is freed once");
    }
}

#[test]
fn a_group_adds_each_part_and_dropping_it_frees_every_block() {
    let mut group = group();
    let first = GhostRig::group_job(0, 1, GhostJobMode::Append, 7, 0);
    let second = GhostRig::group_job(0, 2, GhostJobMode::Append, 7, 1);

    assert_eq!(group.step(&first), GroupStep::Apply);
    group.start(&first);
    assert!(group.add(part(4).layout()));
    assert_eq!(group.step(&second), GroupStep::Apply);
    assert_eq!(group.step(&first), GroupStep::Repeat);
    assert!(group.add(part(8).layout()));

    assert_eq!(group.vertices(), 12);
    group.drop_build();
    assert_eq!(FakeGhost::count(&HEAP_LIVE), 0);
    assert_eq!(FakeGhost::count(&MESH_DATA_DTOR_CALLS), 3);
}

#[test]
#[allow(unsafe_code)]
fn a_part_with_a_field_that_does_not_cover_every_vertex_is_refused_and_the_buffer_stays() {
    let mut group = group();
    group.start(&GhostRig::group_job(0, 1, GhostJobMode::Append, 7, 0));
    assert!(group.add(part(4).layout()));
    let mut uneven = part(4);
    // SAFETY: as in `part`
    unsafe { GameVec::extend(&FakeGhost::heap(), &mut uneven.layout_mut().normals, &[[0.0f32; 4]; 2]) }.unwrap();

    assert!(!group.add(uneven.layout()));
    assert_eq!(group.vertices(), 4);
}

#[test]
fn a_finished_group_is_answered_again_with_its_bytes_until_its_slot_is_freed() {
    let mut group = group();
    let finish = GhostRig::group_job(0, 2, GhostJobMode::Finish, 7, 1);
    group.start(&GhostRig::group_job(0, 1, GhostJobMode::Append, 7, 0));

    group.finish(0, 7, 104);

    assert_eq!(group.step(&finish), GroupStep::RepeatFinish);
    assert_eq!(group.finished_bytes(0), 104);
    group.free(0);
    assert_eq!(group.step(&finish), GroupStep::Ignore);
}
