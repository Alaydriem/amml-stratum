use std::panic::{AssertUnwindSafe, catch_unwind};

use alaydriem_stratum::game::{Tessellator, TessellatorLayout};

use crate::support::FakeGhost;
use crate::support::fake_ghost::{BEGIN_CALLS, END_CALLS, MESH_DTOR_CALLS};

#[test]
#[allow(unsafe_code)]
fn end_with_a_mesh_written_gives_some_and_dropping_it_calls_the_dtor_once() {
    let calls = FakeGhost::calls();
    let mut buffer = FakeGhost::tessellator();
    // SAFETY: the fake buffer is laid out as the Tessellator mirror and outlives the wrapper
    let mut tessellator = unsafe { Tessellator::from_raw(buffer.as_mut_ptr(), &calls) }.unwrap();
    tessellator.begin_quads(24);

    let mesh = tessellator.end("stratum_test");

    let mesh = mesh.expect("the fake end wrote a mesh");
    assert!(mesh.is_valid());
    assert!(!tessellator.is_tessellating());
    assert_eq!(FakeGhost::count(&MESH_DTOR_CALLS), 0);
    drop(mesh);
    assert_eq!(FakeGhost::count(&MESH_DTOR_CALLS), 1);
}

#[test]
#[allow(unsafe_code)]
fn end_with_nothing_begun_gives_none_and_calls_nothing() {
    let calls = FakeGhost::calls();
    let mut buffer = FakeGhost::tessellator();
    // SAFETY: as above
    let mut tessellator = unsafe { Tessellator::from_raw(buffer.as_mut_ptr(), &calls) }.unwrap();

    assert!(tessellator.end("stratum_test").is_none());
    assert_eq!(FakeGhost::count(&END_CALLS), 0);
}

#[test]
#[allow(unsafe_code)]
fn a_build_whose_fill_fails_still_ends() {
    let calls = FakeGhost::calls();
    let mut buffer = FakeGhost::tessellator();
    // SAFETY: as above
    let mut tessellator = unsafe { Tessellator::from_raw(buffer.as_mut_ptr(), &calls) }.unwrap();

    let build = tessellator.build_quads(24, "stratum_test", |_| false).expect("the tessellator was free");

    assert!(!build.filled);
    assert_eq!(FakeGhost::count(&BEGIN_CALLS), 1);
    assert_eq!(FakeGhost::count(&END_CALLS), 1);
    assert!(!tessellator.is_tessellating());
}

#[test]
#[allow(unsafe_code)]
fn a_build_whose_fill_panics_still_ends_and_drops_the_mesh() {
    let calls = FakeGhost::calls();
    let mut buffer = FakeGhost::tessellator();
    // SAFETY: as above
    let mut tessellator = unsafe { Tessellator::from_raw(buffer.as_mut_ptr(), &calls) }.unwrap();

    let result = catch_unwind(AssertUnwindSafe(|| {
        tessellator.build_quads(24, "stratum_test", |_| panic!("a fill that fails part way"))
    }));

    assert!(result.is_err());
    assert_eq!(FakeGhost::count(&BEGIN_CALLS), 1);
    assert_eq!(FakeGhost::count(&END_CALLS), 1);
    assert_eq!(FakeGhost::count(&MESH_DTOR_CALLS), 1);
    assert!(!buffer.view::<TessellatorLayout>().tessellating);
}

#[test]
#[allow(unsafe_code)]
fn a_build_on_a_busy_tessellator_begins_nothing() {
    let calls = FakeGhost::calls();
    let mut buffer = FakeGhost::tessellator();
    buffer.put_u8(0x2A0, 1);
    // SAFETY: as above
    let mut tessellator = unsafe { Tessellator::from_raw(buffer.as_mut_ptr(), &calls) }.unwrap();

    assert!(tessellator.build_quads(24, "stratum_test", |_| true).is_none());
    assert_eq!(FakeGhost::count(&BEGIN_CALLS), 0);
    assert_eq!(FakeGhost::count(&END_CALLS), 0);
}

#[test]
#[allow(unsafe_code)]
fn shift_moves_every_position_by_minus_the_origin() {
    let calls = FakeGhost::calls();
    let mut buffer = FakeGhost::tessellator();
    let mut positions = [[10.0f32, 64.0, -3.0], [11.0, 65.0, -2.0]];
    let first = positions.as_mut_ptr() as u64;
    // the positions vector of mMeshData (Tessellator+8, then +8 in MeshData)
    buffer.put_u64(0x10, first);
    buffer.put_u64(0x18, first + 24);
    buffer.put_u64(0x20, first + 24);
    // SAFETY: as above; the vector points at `positions`, which outlives the wrapper
    let mut tessellator = unsafe { Tessellator::from_raw(buffer.as_mut_ptr(), &calls) }.unwrap();

    tessellator.shift_positions([10.0, 64.0, -3.0]);

    assert_eq!(tessellator.vertex_count(), 2);
    assert_eq!(positions, [[0.0, 0.0, 0.0], [1.0, 1.0, 1.0]]);
}
