use std::ffi::c_void;
use std::sync::atomic::Ordering;

use alaydriem_stratum::game::GameBlockActor;
use amml_game::BlockPos;
use stratum_core::structs::ghost::ChestPair;

use crate::support::FakeBlockActor;
use crate::support::fake_block_actor::{
    COMPONENT_OFFSET, CREATE_CALLS, DELETE_CALLS, DESTROY_CALLS, HELPER_VTABLE, LOAD_CALLS, NBT_READ_CALLS,
    NULL_ELEMENT, NULL_RESULT, NULL_TAG, PAIR_FLAGS_OFFSET, POSITION_OFFSET, READ_BYTES, Seen, TAG_DELETE_SIZE,
    TAG_DELETES, TAG_DTOR_CALLS, TAGS_LIVE,
};

const BLOCK_TYPE: *const c_void = 0xB10C as *const c_void;
const LEVEL: *mut c_void = 0x1E7E1 as *mut c_void;
const POS: BlockPos = BlockPos { x: -6, y: 66, z: -2 };

#[allow(unsafe_code)]
fn load(nbt: Option<&[u8]>) -> Option<GameBlockActor> {
    let calls = FakeBlockActor::calls();
    // SAFETY: fake game code for every call, a fake block type and level the fakes only record
    unsafe { GameBlockActor::load(&calls, BLOCK_TYPE, POS, nbt, LEVEL) }
}

#[test]
fn load_with_nbt_reads_loads_and_destroys_the_tag_once() {
    let actor = load(Some(&[10, 0, 0, 0])).expect("the fake load made an actor");

    assert!(!actor.as_ptr().is_null());
    assert_eq!(FakeBlockActor::count(&NBT_READ_CALLS), 1);
    assert_eq!(*READ_BYTES.lock().unwrap(), vec![10, 0, 0, 0]);
    assert_eq!(FakeBlockActor::count(&LOAD_CALLS), 1);
    assert_eq!(FakeBlockActor::count(&CREATE_CALLS), 0);
    let seen = FakeBlockActor::seen().expect("the load saw its arguments");
    assert_eq!(
        seen,
        Seen {
            block_type: BLOCK_TYPE as usize,
            pos: POS,
            level: LEVEL as usize,
            tag: seen.tag,
            helper_vtable: HELPER_VTABLE.as_ptr() as usize,
        }
    );
    assert_ne!(seen.tag, 0);
    assert_eq!(FakeBlockActor::count(&TAG_DTOR_CALLS), 1);
    assert_eq!(FakeBlockActor::count(&TAG_DELETES), 1);
    assert_eq!(TAG_DELETE_SIZE.load(Ordering::SeqCst), 0x18);
    assert_eq!(TAGS_LIVE.load(Ordering::SeqCst), 0);
}

#[test]
fn load_without_nbt_uses_the_factory() {
    let actor = load(None).expect("the fake factory made an actor");

    assert!(!actor.as_ptr().is_null());
    assert_eq!(FakeBlockActor::count(&CREATE_CALLS), 1);
    assert_eq!(FakeBlockActor::count(&LOAD_CALLS), 0);
    assert_eq!(FakeBlockActor::count(&NBT_READ_CALLS), 0);
    let seen = FakeBlockActor::seen().expect("the factory saw its arguments");
    assert_eq!((seen.block_type, seen.pos), (BLOCK_TYPE as usize, POS));
}

#[test]
fn empty_nbt_uses_the_factory() {
    let actor = load(Some(&[]));

    assert!(actor.is_some());
    assert_eq!(FakeBlockActor::count(&CREATE_CALLS), 1);
    assert_eq!(FakeBlockActor::count(&NBT_READ_CALLS), 0);
}

#[test]
fn a_null_result_gives_none() {
    NULL_RESULT.store(true, Ordering::SeqCst);

    assert!(load(None).is_none());
    assert!(load(Some(&[10, 0, 0, 0])).is_none());

    assert_eq!(FakeBlockActor::count(&TAG_DTOR_CALLS), 1);
    assert_eq!(FakeBlockActor::count(&TAG_DELETES), 1);
    assert_eq!(FakeBlockActor::count(&DESTROY_CALLS), 0);
}

#[test]
#[allow(unsafe_code)]
fn a_null_block_type_or_a_load_with_no_level_calls_nothing() {
    let calls = FakeBlockActor::calls();

    // SAFETY: fake game code; the wrapper refuses before any call
    let no_type = unsafe { GameBlockActor::load(&calls, std::ptr::null(), POS, None, LEVEL) };
    // SAFETY: as above
    let no_level = unsafe { GameBlockActor::load(&calls, BLOCK_TYPE, POS, Some(&[10, 0]), std::ptr::null_mut()) };

    assert!(no_type.is_none());
    assert!(no_level.is_none());
    assert_eq!(FakeBlockActor::count(&CREATE_CALLS), 0);
    assert_eq!(FakeBlockActor::count(&NBT_READ_CALLS), 0);
}

#[test]
fn dropping_an_actor_releases_it_once() {
    let actor = load(None).unwrap();

    assert_eq!(FakeBlockActor::count(&DESTROY_CALLS), 0);
    drop(actor);
    assert_eq!(FakeBlockActor::count(&DESTROY_CALLS), 1);
    assert_eq!(FakeBlockActor::count(&DELETE_CALLS), 1);
}

#[test]
fn the_render_component_is_the_one_the_actor_gives() {
    let calls = FakeBlockActor::calls();
    let actor = load(None).unwrap();

    let component = actor.render_component(&calls);

    assert_eq!(component as usize, actor.as_ptr() as usize + COMPONENT_OFFSET);
}

#[test]
fn a_read_that_gives_no_tag_frees_nothing_and_gives_none() {
    NULL_TAG.store(true, Ordering::SeqCst);

    let actor = load(Some(&[10, 0, 0, 0]));

    assert!(actor.is_none());
    assert_eq!(FakeBlockActor::count(&NBT_READ_CALLS), 1);
    assert_eq!(FakeBlockActor::count(&LOAD_CALLS), 0);
    assert_eq!(FakeBlockActor::count(&TAG_DTOR_CALLS), 0);
    assert_eq!(FakeBlockActor::count(&TAG_DELETES), 0);
}

#[test]
fn a_result_with_a_control_block_and_no_element_is_released_once() {
    NULL_ELEMENT.store(true, Ordering::SeqCst);

    assert!(load(None).is_none());
    assert_eq!(FakeBlockActor::count(&DESTROY_CALLS), 1);
    assert_eq!(FakeBlockActor::count(&DELETE_CALLS), 1);

    assert!(load(Some(&[10, 0, 0, 0])).is_none());
    assert_eq!(FakeBlockActor::count(&DESTROY_CALLS), 2);
    assert_eq!(FakeBlockActor::count(&TAG_DTOR_CALLS), 1);
}

#[test]
#[allow(unsafe_code)]
fn a_lead_actor_points_at_itself_with_the_partner_position() {
    let calls = FakeBlockActor::calls().chest.unwrap();
    // the actor's own mPosition, not the position of the make, is the base of the partner position
    let own = BlockPos { x: 100, y: 70, z: -50 };
    let cases = [
        (ChestPair::PlusX, BlockPos { x: 101, y: 70, z: -50 }, 0xFF, 0xFD),
        (ChestPair::MinusX, BlockPos { x: 99, y: 70, z: -50 }, 0xFF, 0xFD),
        (ChestPair::PlusZ, BlockPos { x: 100, y: 70, z: -49 }, 0x00, 0x02),
        (ChestPair::MinusZ, BlockPos { x: 100, y: 70, z: -51 }, 0x00, 0x02),
    ];
    for (pair, partner, flags_before, flags_after) in cases {
        let mut actor = load(None).unwrap();
        let base = actor.as_ptr().cast::<u8>();
        // SAFETY: a fake actor larger than the fields
        unsafe {
            base.add(POSITION_OFFSET).cast::<BlockPos>().write_unaligned(own);
            base.add(PAIR_FLAGS_OFFSET).write(flags_before);
        }

        // SAFETY: fake game code and a fake actor laid out as a chest
        unsafe { actor.set_large_chest(&calls, pair) };

        let (paired, position, flags) = FakeBlockActor::large_chest(actor.as_ptr() as usize);
        assert_eq!(paired, actor.as_ptr() as usize, "{pair:?}: the partner pointer is the actor itself");
        assert_eq!(position, partner, "{pair:?}");
        assert_eq!(flags, flags_after, "{pair:?}: only the pair axis bit changes");
    }
}
