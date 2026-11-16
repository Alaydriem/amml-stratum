use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicUsize, Ordering};

use alaydriem_stratum::game::StringByteInputLayout;
use alaydriem_stratum::ghost::SkullSlot;
use alaydriem_stratum::types::{BlockActorCalls, ChestPairCalls, SkullCalls};
use amml_game::BlockPos;
use amml_msvc::{RefCountBase, RefCountVtable};
use stratum_core::structs::ghost::SkullPose;

/// Fake game code for the block-actor wrapper: the factory, `loadStatic`, `NbtIo::read`, the tag destructor, the game
/// heap and the actor's control block, with counters. Each test runs in its own process (nextest), so the counters
/// start at zero.
pub struct FakeBlockActor;

pub static CREATE_CALLS: AtomicU32 = AtomicU32::new(0);
pub static LOAD_CALLS: AtomicU32 = AtomicU32::new(0);
pub static NBT_READ_CALLS: AtomicU32 = AtomicU32::new(0);
pub static TAG_DTOR_CALLS: AtomicU32 = AtomicU32::new(0);
pub static TAG_DELETES: AtomicU32 = AtomicU32::new(0);
/// The size the fake heap delete got last.
pub static TAG_DELETE_SIZE: AtomicUsize = AtomicUsize::new(0);
/// Tags the fake read made and the wrapper has not destroyed yet.
pub static TAGS_LIVE: AtomicI32 = AtomicI32::new(0);
pub static DESTROY_CALLS: AtomicU32 = AtomicU32::new(0);
pub static DELETE_CALLS: AtomicU32 = AtomicU32::new(0);
/// When set, the factory and `loadStatic` give an empty pointer, as the game does for a type with no actor.
pub static NULL_RESULT: AtomicBool = AtomicBool::new(false);
/// When set, the factory and `loadStatic` give a live control block with a null element.
pub static NULL_ELEMENT: AtomicBool = AtomicBool::new(false);
/// When set, the fake actor answers false to `isType(Chest)`, as an actor of another class does.
pub static NOT_A_CHEST: AtomicBool = AtomicBool::new(false);
/// The calls of the fake `isType` and the last type it was asked for.
pub static IS_TYPE_CALLS: AtomicU32 = AtomicU32::new(0);
pub static IS_TYPE_ASKED: AtomicU32 = AtomicU32::new(u32::MAX);
/// When set, the fake read gives no tag.
pub static NULL_TAG: AtomicBool = AtomicBool::new(false);
/// The bytes the fake read saw through the input.
pub static READ_BYTES: Mutex<Vec<u8>> = Mutex::new(Vec::new());
/// What the fake factory or `loadStatic` got: the block type, the position and, for a load, the level, the tag and
/// the vtable of the data load helper.
pub static SEEN: Mutex<Option<Seen>> = Mutex::new(None);

/// The skull slot the drawer is given; the fake dispatcher render reads it at each call.
pub static SKULL_SLOT: SkullSlot = SkullSlot::new();

/// Every call the fake `BlockActorRenderDispatcher::render` got, oldest first.
pub static RENDERS: Mutex<Vec<Rendered>> = Mutex::new(Vec::new());

/// The arguments of one fake dispatcher render, with the buffers it was given read at the call.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rendered {
    pub dispatcher: usize,
    pub ctx: usize,
    pub region: usize,
    pub component: usize,
    pub block: usize,
    pub render_pos: [f32; 3],
    pub world_pos: BlockPos,
    pub alpha: u8,
    pub material: [u64; 2],
    pub texture: usize,
    pub breaking: i32,
    pub metadata_engaged: u8,
    /// What `SKULL_SLOT` held during the call: the pose and the name pointer.
    pub skull: Option<(SkullPose, usize)>,
}

/// The arguments of the last make or load.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seen {
    pub block_type: usize,
    pub pos: BlockPos,
    pub level: usize,
    pub tag: usize,
    pub helper_vtable: usize,
}

/// A fake block actor: its vtable, then room past the chest pair fields at the offsets of the game's
/// `ChestBlockActor`. The make writes its position into `mPosition`.
#[repr(C)]
pub struct ActorObject {
    pub vtable: *const usize,
    pub body: [u8; 0x320],
}

/// The vtable slot of `_getRenderComponent` in the fake actor.
pub const RENDER_COMPONENT_SLOT: u64 = 21;
/// The vtable slot of `isType` in the fake actor.
pub const IS_TYPE_SLOT: u64 = 16;
/// The offset of the render component in the fake actor, as the game's actors give it.
pub const COMPONENT_OFFSET: usize = 0x40;
/// The offsets of `BlockActor::mPosition` and the chest pair fields in the fake actor, as the game's chest gives them.
pub const POSITION_OFFSET: usize = 0x8;
pub const PAIR_FLAGS_OFFSET: usize = 0x2f8;
pub const PAIRED_OFFSET: usize = 0x308;
pub const PAIRED_POSITION_OFFSET: usize = 0x310;
/// The offset of `BlockType::mNameInfo.mFullName`, as the game gives it.
pub const FULL_NAME_OFFSET: u64 = 0xe0;
/// The fake StringByteInput and DefaultDataLoadHelper vtables: addresses the fakes check.
pub static INPUT_VTABLE: [usize; 2] = [0, 0];
pub static HELPER_VTABLE: [usize; 2] = [0, 0];

static CONTROL_VTABLE: RefCountVtable = RefCountVtable {
    destroy: FakeBlockActor::destroy,
    delete_this: FakeBlockActor::delete_this,
    destructor: FakeBlockActor::control_destructor,
    get_deleter: FakeBlockActor::get_deleter,
};

#[allow(unsafe_code)]
impl FakeBlockActor {
    pub fn calls() -> BlockActorCalls {
        BlockActorCalls {
            render_block_entities_body: 0x5369340,
            level_renderer_player_vtable: 0xf247fb0,
            dispatcher_render: Self::dispatcher_render as *const () as usize,
            context_dispatcher: 0x30,
            context_impl: 0xa8,
            impl_camera_target: 0x20,
            create_block_actor: Self::create as *const () as usize,
            load_static: Self::load_static as *const () as usize,
            render_component_slot: RENDER_COMPONENT_SLOT,
            data_load_helper_vtable: HELPER_VTABLE.as_ptr() as usize,
            get_level_slot: 0xa6,
            string_byte_input_vtable: INPUT_VTABLE.as_ptr() as usize,
            nbt_read: Self::nbt_read as *const () as usize,
            compound_tag_dtor: Self::tag_dtor as *const () as usize,
            heap_delete: Self::heap_delete,
            chest: Some(ChestPairCalls {
                is_type_slot: IS_TYPE_SLOT,
                actor_position: POSITION_OFFSET as u64,
                chest_pair_flags: PAIR_FLAGS_OFFSET as u64,
                chest_large_paired: PAIRED_OFFSET as u64,
                chest_large_paired_position: PAIRED_POSITION_OFFSET as u64,
            }),
            skull: Some(SkullCalls { render_skull: 0x6a5ea50, block_type_full_name: FULL_NAME_OFFSET }),
        }
    }

    /// The chest pair fields of the fake actor at `actor`: the partner pointer, the partner position and the flags
    /// byte.
    pub fn large_chest(actor: usize) -> (usize, BlockPos, u8) {
        let base = actor as *const u8;
        // SAFETY: a fake actor this file made, larger than every chest field
        unsafe {
            (
                base.add(PAIRED_OFFSET).cast::<usize>().read_unaligned(),
                base.add(PAIRED_POSITION_OFFSET).cast::<BlockPos>().read_unaligned(),
                base.add(PAIR_FLAGS_OFFSET).read(),
            )
        }
    }

    /// The actor whose render component the dispatcher got in `render`.
    pub fn actor_of(render: &Rendered) -> usize {
        render.component - COMPONENT_OFFSET
    }

    pub fn count(counter: &AtomicU32) -> u32 {
        counter.load(Ordering::SeqCst)
    }

    pub fn seen() -> Option<Seen> {
        *SEEN.lock().unwrap()
    }

    // a new fake actor at `pos` with one use and its control block; both leak for the test process
    fn leak_actor(pos: BlockPos) -> [usize; 2] {
        if NULL_RESULT.load(Ordering::SeqCst) {
            return [0, 0];
        }
        let control = Box::leak(Box::new(RefCountBase {
            vtable: &CONTROL_VTABLE,
            uses: AtomicI32::new(1),
            weaks: AtomicI32::new(1),
        }));
        if NULL_ELEMENT.load(Ordering::SeqCst) {
            return [0, control as *mut RefCountBase as usize];
        }
        let mut table = [0usize; 24];
        table[RENDER_COMPONENT_SLOT as usize] = Self::render_component as *const () as usize;
        table[IS_TYPE_SLOT as usize] = Self::is_type as *const () as usize;
        let table = Box::leak(Box::new(table));
        let actor = Box::leak(Box::new(ActorObject { vtable: table.as_ptr(), body: [0; 0x320] }));
        let base = (actor as *mut ActorObject).cast::<u8>();
        // SAFETY: the fake actor holds mPosition at its offset
        unsafe { base.add(POSITION_OFFSET).cast::<BlockPos>().write_unaligned(pos) };
        [actor as *mut ActorObject as usize, control as *mut RefCountBase as usize]
    }

    // shared_ptr<BlockActor> VanillaBlockActorFactory::createBlockActor(BlockPos const&, BlockType const&)
    extern "C" fn create(out: *mut [usize; 2], pos: *const BlockPos, block_type: *const c_void) -> *mut [usize; 2] {
        CREATE_CALLS.fetch_add(1, Ordering::SeqCst);
        // SAFETY: the wrapper passes a live position
        let pos = unsafe { *pos };
        *SEEN.lock().unwrap() = Some(Seen { block_type: block_type as usize, pos, level: 0, tag: 0, helper_vtable: 0 });
        // SAFETY: the wrapper passes a buffer for a shared_ptr
        unsafe { *out = Self::leak_actor(pos) };
        out
    }

    // shared_ptr<BlockActor> BlockActor::loadStatic(BlockType const&, BlockPos const&, ILevel&, CompoundTag const&,
    // DataLoadHelper&)
    extern "C" fn load_static(
        out: *mut [usize; 2],
        block_type: *const c_void,
        pos: *const BlockPos,
        level: *mut c_void,
        tag: *const c_void,
        helper: *mut c_void,
    ) -> *mut [usize; 2] {
        LOAD_CALLS.fetch_add(1, Ordering::SeqCst);
        assert_eq!(TAGS_LIVE.load(Ordering::SeqCst), 1, "the tag is live during the load");
        // SAFETY: the wrapper passes a live position and a helper object whose first qword is its vtable
        let (pos, helper_vtable) = unsafe { (*pos, *helper.cast::<usize>()) };
        *SEEN.lock().unwrap() = Some(Seen {
            block_type: block_type as usize,
            pos,
            level: level as usize,
            tag: tag as usize,
            helper_vtable,
        });
        // SAFETY: the wrapper passes a buffer for a shared_ptr
        unsafe { *out = Self::leak_actor(pos) };
        out
    }

    // std::unique_ptr<CompoundTag> NbtIo::read(IDataInput&): a new 0x18-byte fake tag
    extern "C" fn nbt_read(out: *mut *mut c_void, input: *mut StringByteInputLayout) -> *mut *mut c_void {
        NBT_READ_CALLS.fetch_add(1, Ordering::SeqCst);
        // SAFETY: the wrapper passes a live input over live bytes
        unsafe {
            let input = &*input;
            assert_eq!(input.vtable as usize, INPUT_VTABLE.as_ptr() as usize, "the input has the input vtable");
            *READ_BYTES.lock().unwrap() = std::slice::from_raw_parts(input.data, input.len).to_vec();
        }
        if NULL_TAG.load(Ordering::SeqCst) {
            // SAFETY: the wrapper passes a pointer-sized result buffer
            unsafe { *out = std::ptr::null_mut() };
            return out;
        }
        TAGS_LIVE.fetch_add(1, Ordering::SeqCst);
        let tag = Box::leak(Box::new([0u64; 3]));
        // SAFETY: the wrapper passes a pointer-sized result buffer
        unsafe { *out = tag.as_mut_ptr().cast() };
        out
    }

    extern "C" fn tag_dtor(_tag: *mut c_void) {
        TAG_DTOR_CALLS.fetch_add(1, Ordering::SeqCst);
        TAGS_LIVE.fetch_sub(1, Ordering::SeqCst);
    }

    pub unsafe extern "C" fn heap_delete(_block: *mut u8, size: usize) {
        TAG_DELETES.fetch_add(1, Ordering::SeqCst);
        TAG_DELETE_SIZE.store(size, Ordering::SeqCst);
    }

    /// Every fake dispatcher render so far.
    pub fn renders() -> Vec<Rendered> {
        RENDERS.lock().unwrap().clone()
    }

    // void BlockActorRenderDispatcher::render(BaseActorRenderContext&, BlockSource&,
    // IVanillaRenderBlockActorComponent&, Block const*, Vec3 const&, BlockPos const&, bool, MaterialPtr const*,
    // ClientTexture const*, int, std::optional<RenderMetadata>*)
    #[allow(clippy::too_many_arguments)]
    extern "C" fn dispatcher_render(
        dispatcher: *mut c_void,
        ctx: *mut c_void,
        region: *mut c_void,
        component: *mut c_void,
        block: *const c_void,
        render_pos: *const [f32; 3],
        world_pos: *const BlockPos,
        alpha: u8,
        material: *const [u64; 2],
        texture: *const c_void,
        breaking: i32,
        metadata: *mut [u8; 0x40],
    ) {
        // SAFETY: the drawer passes live positions and its live material and metadata buffers
        let (render_pos, world_pos, material, metadata_engaged) =
            unsafe { (*render_pos, *world_pos, *material, (*metadata)[0x38]) };
        RENDERS.lock().unwrap().push(Rendered {
            dispatcher: dispatcher as usize,
            ctx: ctx as usize,
            region: region as usize,
            component: component as usize,
            block: block as usize,
            render_pos,
            world_pos,
            alpha,
            material,
            texture: texture as usize,
            breaking,
            metadata_engaged,
            skull: SKULL_SLOT.get().map(|(pose, name)| (pose, name as usize)),
        });
    }

    // IVanillaRenderBlockActorComponent* BlockActor::_getRenderComponent()
    extern "C" fn render_component(this: *mut c_void) -> *mut c_void {
        // SAFETY: the wrapper passes a fake actor, larger than the offset
        unsafe { this.cast::<u8>().add(COMPONENT_OFFSET).cast() }
    }

    // bool BlockActor::isType(BlockActorType) const: a fake chest answers true for 2, unless NOT_A_CHEST is set
    extern "C" fn is_type(_this: *const c_void, kind: u8) -> u8 {
        IS_TYPE_CALLS.fetch_add(1, Ordering::SeqCst);
        IS_TYPE_ASKED.store(u32::from(kind), Ordering::SeqCst);
        u8::from(kind == 2 && !NOT_A_CHEST.load(Ordering::SeqCst))
    }

    unsafe extern "C" fn destroy(_this: *mut RefCountBase) {
        DESTROY_CALLS.fetch_add(1, Ordering::SeqCst);
    }

    unsafe extern "C" fn delete_this(_this: *mut RefCountBase) {
        DELETE_CALLS.fetch_add(1, Ordering::SeqCst);
    }

    unsafe extern "C" fn control_destructor(this: *mut RefCountBase, _flags: u32) -> *mut c_void {
        this.cast()
    }

    unsafe extern "C" fn get_deleter(_this: *const RefCountBase, _info: *const c_void) -> *mut c_void {
        std::ptr::null_mut()
    }
}
