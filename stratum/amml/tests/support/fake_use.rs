use std::ffi::c_void;
use std::mem::MaybeUninit;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};

use alaydriem_stratum::easy_place::UseHooks;
use alaydriem_stratum::types::{PacketCalls, PlayerCalls, UseCalls, UseFrame};
use amml_game::{
    BlockPos, ContainerClosePacketLayout, ContainerOpenPacketLayout, CreativeItemEntryLayout, HitResultLayout,
    ItemInstanceLayout, ItemStackLayout, ItemStackResponseContainerInfoLayout, ItemStackResponseInfoLayout,
    ItemStackResponsePacketLayout, ItemStackResponseSlotInfoLayout, StopLatch,
};

use crate::support::FakePacket;
use crate::support::fake_packet::NAME_BY_ID;

/// Fake game objects for the easy place use path, laid out as the mirrors and calls read them: a client, its local
/// player with an inventory of 36 stacks, a game mode, a level with its hit result, a creative registry, a region
/// whose blocks are all air but one stone cell, and a HUD. Every game call is logged in order. Each test runs in its
/// own process (nextest), so the statics start empty.
pub struct FakeUse {
    pub client: *mut c_void,
    pub player: *mut c_void,
    pub supplies: *mut c_void,
    pub items: *mut ItemStackLayout,
    pub game_mode: *mut c_void,
    pub hit: *mut HitResultLayout,
    pub region: *mut c_void,
    pub hud: usize,
    pub calls: UseCalls,
    pub players: PlayerCalls,
    pub packets: PacketCalls,
}

/// The game calls in the order they ran.
pub static LOG: Mutex<Vec<String>> = Mutex::new(Vec::new());
/// Whether the fake `_handleSlotSelection` selects the slot it is given.
pub static SELECT_TAKES: AtomicBool = AtomicBool::new(true);
/// Whether the fake `buildBlock` builds.
pub static BUILD_OK: AtomicBool = AtomicBool::new(true);
pub static ITEM_DTOR_CALLS: AtomicU32 = AtomicU32::new(0);
pub static STACK_DTOR_CALLS: AtomicU32 = AtomicU32::new(0);
/// The hooks the fake `buildBlock` asks first, as the build detour does; none when 0.
pub static BUILD_HOOKS: AtomicUsize = AtomicUsize::new(0);

// the fake items: the address of each SharedCounter<Item>, whose first member points to the Item
static STONE_COUNTER: AtomicUsize = AtomicUsize::new(0);
static DIRT_COUNTER: AtomicUsize = AtomicUsize::new(0);
static PLANKS_COUNTER: AtomicUsize = AtomicUsize::new(0);
// the one stone cell of the region, and the stone block the palette gives
static STONE_CELL: Mutex<Option<[i32; 3]>> = Mutex::new(None);
static BLOCKS: Mutex<[usize; 2]> = Mutex::new([0, 0]);
static SUPPLIES: AtomicUsize = AtomicUsize::new(0);
static STACK_VTABLE: AtomicUsize = AtomicUsize::new(0);
static LATCH: StopLatch = StopLatch::new();

#[allow(unsafe_code)]
impl FakeUse {
    pub const SELECTED: u64 = 0x10;
    pub const CONTAINER: u64 = 0xB8;
    pub const ITEMS: u64 = 0x198;
    const PLAYER_INVENTORY: u64 = 0x10;
    const GAME_MODE: u64 = 0x10 + 0xAA0 - 0x5B8;
    const AABB: u64 = 0x20;
    const LEVEL: u64 = 0x28;
    const LOCAL_PLAYER_SLOT: u64 = 5;
    const HIT_RESULT_SLOT: u64 = 3;
    const ITEM_REGISTRY_SLOT: u64 = 4;
    const CAN_BE_BUILT_OVER_SLOT: u64 = 2;
    const IS_INTERACTIVE_SLOT: u64 = 3;
    const AS_ITEM_INSTANCE_SLOT: u64 = 4;
    /// The creative net id of planks, the item the palette's block 7 gives.
    pub const PLANKS_NET_ID: u32 = 41;
    pub const PLANKS_PALETTE_ID: u16 = 7;

    /// The rig: the player stands at (0.5, 64, 0.5) looking north (-z) along the floor, the region holds stone at
    /// (0, 64, -3) only, slot 0 holds 10 dirt and slot 9 holds 64 stone.
    pub fn new() -> Self {
        NAME_BY_ID.store(true, Ordering::SeqCst);
        for counter in [&STONE_COUNTER, &DIRT_COUNTER, &PLANKS_COUNTER] {
            let item = Self::bytes(0x10) as usize;
            counter.store(Box::leak(Box::new([item])).as_ptr() as usize, Ordering::SeqCst);
        }
        *STONE_CELL.lock().unwrap() = Some([0, 64, -3]);
        let stack_table =
            Self::table(&[(0, Self::stack_dtor as *const () as usize), (3, Self::reinit as *const () as usize)]);
        STACK_VTABLE.store(stack_table as usize, Ordering::SeqCst);

        let block_type = Self::object(
            0x10,
            &[
                (Self::CAN_BE_BUILT_OVER_SLOT, Self::can_be_built_over as *const () as usize),
                (Self::IS_INTERACTIVE_SLOT, Self::is_interactive as *const () as usize),
                (Self::AS_ITEM_INSTANCE_SLOT, Self::as_item_instance as *const () as usize),
            ],
        );
        let air = Self::bytes(0x70);
        let stone = Self::bytes(0x70);
        // SAFETY: two fake blocks of 0x70 bytes; mBlockType is at 0x68
        unsafe {
            Self::poke(air, 0x68, block_type as usize);
            Self::poke(stone, 0x68, block_type as usize);
        }
        *BLOCKS.lock().unwrap() = [air as usize, stone as usize];

        let items = Box::leak(Box::new([const { MaybeUninit::<ItemStackLayout>::zeroed() }; 36]))
            .as_mut_ptr()
            .cast::<ItemStackLayout>();
        let container = Self::object(
            Self::ITEMS as usize + 0x18,
            &[(12, Self::set_item as *const () as usize), (44, Self::swap_slots as *const () as usize)],
        );
        let supplies = Self::bytes(0xC0);
        let game_mode = Self::object(0xB0, &[(6, Self::build_block as *const () as usize)]);
        let hit = Self::bytes(0x80).cast::<HitResultLayout>();
        let level = Self::object(
            0x10,
            &[
                (Self::HIT_RESULT_SLOT, Self::hit_result as *const () as usize),
                (Self::ITEM_REGISTRY_SLOT, Self::item_registry as *const () as usize),
            ],
        );
        let aabb = Self::bytes(0x18);
        let player = Self::bytes(0x600);
        let client = Self::object(
            0x10,
            &[
                (3, FakePacket::get_packet_sender as *const () as usize),
                (Self::LOCAL_PLAYER_SLOT, Self::local_player as *const () as usize),
            ],
        );
        // SAFETY: each fake object is large enough for the offsets written into it
        unsafe {
            Self::poke(container, Self::ITEMS, [items as usize, items.add(36) as usize, items.add(36) as usize]);
            Self::poke(supplies, Self::CONTAINER, container as usize);
            Self::poke(game_mode, 0x8, player as usize);
            Self::poke(game_mode, 0xA8, 200i64);
            Self::poke(level, 0x8, hit as usize);
            Self::poke(aabb, 0, [0.2f32, 64.0, 0.2, 0.8, 65.8, 0.8]);
            Self::poke(player, Self::PLAYER_INVENTORY, supplies as usize);
            Self::poke(player, Self::GAME_MODE, game_mode as usize);
            Self::poke(player, Self::AABB, aabb as usize);
            Self::poke(player, Self::LEVEL, level as usize);
            Self::poke(client, 0x8, player as usize);
        }
        SUPPLIES.store(supplies as usize, Ordering::SeqCst);
        let rig = Self {
            client,
            player,
            supplies,
            items,
            game_mode,
            hit,
            region: Self::bytes(0x10),
            hud: Self::bytes(0x10) as usize,
            calls: UseCalls {
                local_player_slot: Self::LOCAL_PLAYER_SLOT,
                game_mode_player: 0x8,
                build_block_slot: 6,
                get_max_pick_range: Self::max_pick_range as *const () as usize,
                get_view_vector: Self::view_vector as *const () as usize,
                get_attach_pos: Self::attach_pos as *const () as usize,
                aabb_shape_component: Self::AABB,
                level_get_hit_result_slot: Self::HIT_RESULT_SLOT,
                block_source_get_block: Self::get_block as *const () as usize,
                can_be_built_over_slot: Self::CAN_BE_BUILT_OVER_SLOT,
                is_interactive_block_slot: Self::IS_INTERACTIVE_SLOT,
                as_item_instance_slot: Self::AS_ITEM_INSTANCE_SLOT,
                item_stack_base_dtor: Self::item_dtor as *const () as usize,
                handle_slot_selection: Self::handle_slot_selection as *const () as usize,
            },
            players: PlayerCalls {
                player_inventory: Self::PLAYER_INVENTORY,
                swap_slots_slot: 44,
                set_item_slot: 12,
                item_stack_empty_ctor: Self::stack_ctor as *const () as usize,
                item_stack_copy_ctor: Self::stack_copy as *const () as usize,
                item_stack_vtable: stack_table as usize,
                item_stack_reinit_slot: 3,
                get_max_stack_size: Self::max_stack_size as *const () as usize,
                actor_level: Self::LEVEL,
                level_get_item_registry_slot: Self::ITEM_REGISTRY_SLOT,
                get_creative_item_registry: Self::creative_registry as *const () as usize,
                creative_items: 0x18,
                get_runtime_id: Self::runtime_id as *const () as usize,
                get_player_game_type: Self::game_type as *const () as usize,
            },
            packets: FakePacket::calls(),
        };
        rig.fill(0, Self::dirt(), 10);
        rig.fill(9, Self::stone(), 64);
        rig
    }

    pub fn stone() -> usize {
        STONE_COUNTER.load(Ordering::SeqCst)
    }

    pub fn dirt() -> usize {
        DIRT_COUNTER.load(Ordering::SeqCst)
    }

    pub fn planks() -> usize {
        PLANKS_COUNTER.load(Ordering::SeqCst)
    }

    /// The Item of an item counter.
    pub fn item(counter: usize) -> *const c_void {
        if counter == 0 {
            return std::ptr::null();
        }
        // SAFETY: a fake counter this rig made, the Item pointer first
        unsafe { *(counter as *const *const c_void) }
    }

    /// The stone block, as the palette gives it for `PLANKS_PALETTE_ID`.
    pub fn palette_block(id: u16) -> *const c_void {
        if id == Self::PLANKS_PALETTE_ID { BLOCKS.lock().unwrap()[1] as *const c_void } else { std::ptr::null() }
    }

    /// Puts `count` items of the counter `item` into `slot`, with the net id 100 + slot.
    pub fn fill(&self, slot: usize, item: usize, count: u8) {
        // SAFETY: a stack of the fake container
        unsafe { Self::write_stack(self.items.add(slot), item, count, 100 + slot as i32) };
    }

    /// The item counter and count of `slot`.
    pub fn slot(&self, slot: usize) -> (usize, u8) {
        // SAFETY: a stack of the fake container
        unsafe {
            let stack = self.items.add(slot);
            (Self::peek::<usize>(stack.cast(), 0x08), (*stack).base.count)
        }
    }

    /// The net id of `slot`.
    pub fn net_id(&self, slot: usize) -> Option<i32> {
        // SAFETY: a stack of the fake container
        unsafe { (*self.items.add(slot)).net_id_variant.net_id() }
    }

    pub fn selected(&self) -> i32 {
        // SAFETY: the fake PlayerInventory holds mSelected at SELECTED
        unsafe { Self::peek::<i32>(self.supplies, Self::SELECTED) }
    }

    /// A frame of the rig with the key held at `now`.
    pub fn frame(&self, now: f64) -> UseFrame {
        UseFrame { client: self.client, player: self.player, region: self.region, key_held: true, now }
    }

    /// Hooks started with the rig's calls.
    pub fn hooks(&self) -> &'static UseHooks {
        let hooks: &'static UseHooks = Box::leak(Box::new(UseHooks::new()));
        hooks.start(&self.packets, &self.players, &self.calls);
        hooks
    }

    /// The fake buildBlock asks `hooks` first, as the build detour does.
    pub fn build_through(hooks: &'static UseHooks) {
        BUILD_HOOKS.store(std::ptr::from_ref(hooks) as usize, Ordering::SeqCst);
    }

    /// A game mode whose player is `player`.
    pub fn game_mode_of(player: usize) -> *mut c_void {
        let game_mode = Self::object(0xB0, &[(6, Self::build_block as *const () as usize)]);
        // SAFETY: a fake game mode of 0xB0 bytes
        unsafe { Self::poke(game_mode, 0x8, player) };
        game_mode
    }

    pub fn log() -> Vec<String> {
        LOG.lock().unwrap().clone()
    }

    pub fn clear_log() {
        LOG.lock().unwrap().clear();
    }

    pub fn count(counter: &AtomicU32) -> u32 {
        counter.load(Ordering::SeqCst)
    }

    /// An ItemStackResponse packet with one response: the request `id`, `result`, and, when given, one hotbar
    /// container with one slot and its net id.
    pub fn response(id: i32, result: u8, hotbar: Option<(u8, i32)>) -> &'static ItemStackResponsePacketLayout {
        // SAFETY: zeroed mirrors whose vectors point into leaked arrays; every member a test reads is written
        unsafe {
            let slot = Box::leak(Box::new(MaybeUninit::<ItemStackResponseSlotInfoLayout>::zeroed())).as_mut_ptr();
            let container =
                Box::leak(Box::new(MaybeUninit::<ItemStackResponseContainerInfoLayout>::zeroed())).as_mut_ptr();
            let info = Box::leak(Box::new(MaybeUninit::<ItemStackResponseInfoLayout>::zeroed())).as_mut_ptr();
            let packet = Box::leak(Box::new(MaybeUninit::<ItemStackResponsePacketLayout>::zeroed())).as_mut_ptr();
            if let Some((index, net_id)) = hotbar {
                (*slot).slot = index;
                (*slot).item_stack_net_id = net_id;
                (*container).container_name = 28;
                (*container).slots.set_raw_parts(slot, slot.add(1), slot.add(1));
                (*info).containers.set_raw_parts(container, container.add(1), container.add(1));
            }
            (*info).result = result;
            (*info).client_request_id = id;
            (*packet).responses.set_raw_parts(info, info.add(1), info.add(1));
            &*packet
        }
    }

    /// A ContainerOpen packet of `container_id` and `container_type` (the inventory is -1).
    pub fn open_packet(container_id: i8, container_type: i8) -> &'static ContainerOpenPacketLayout {
        // SAFETY: a zeroed mirror whose two members are written
        unsafe {
            let packet = Box::leak(Box::new(MaybeUninit::<ContainerOpenPacketLayout>::zeroed())).as_mut_ptr();
            (*packet).container_id = container_id;
            (*packet).container_type = container_type;
            &*packet
        }
    }

    /// A ContainerClose packet of `container_id`.
    pub fn close_packet(container_id: i8) -> &'static ContainerClosePacketLayout {
        // SAFETY: a zeroed mirror whose id is written
        unsafe {
            let packet = Box::leak(Box::new(MaybeUninit::<ContainerClosePacketLayout>::zeroed())).as_mut_ptr();
            (*packet).container_id = container_id;
            &*packet
        }
    }

    /// A creative registry with one entry: planks with `PLANKS_NET_ID`.
    fn registry() -> usize {
        static REGISTRY: AtomicUsize = AtomicUsize::new(0);
        if REGISTRY.load(Ordering::SeqCst) == 0 {
            let entries =
                Box::leak(Box::new([const { MaybeUninit::<CreativeItemEntryLayout>::zeroed() }; 1])).as_mut_ptr();
            // SAFETY: one zeroed entry; the item counter sits at mItemInstance (0x28) + 8
            unsafe {
                Self::poke(entries.cast(), 0x28 + 0x08, Self::planks());
                (*entries.cast::<CreativeItemEntryLayout>()).creative_net_id = Self::PLANKS_NET_ID;
            }
            let registry = Self::bytes(0x30);
            // SAFETY: a fake registry of 0x30 bytes holds its entry vector at 0x18
            unsafe { Self::poke(registry, 0x18, [entries as usize, entries.add(1) as usize]) };
            REGISTRY.store(registry as usize, Ordering::SeqCst);
        }
        REGISTRY.load(Ordering::SeqCst)
    }

    fn push(line: String) {
        LOG.lock().unwrap().push(line);
    }

    unsafe fn write_stack(stack: *mut ItemStackLayout, item: usize, count: u8, net_id: i32) {
        // SAFETY: the caller passes a stack of 152 bytes; mItem is at 8 and mCount at 0x22
        unsafe {
            Self::poke(stack.cast(), 0x08, item);
            (*stack).base.count = count;
            (*stack).net_id_variant = amml_game::ItemStackNetIdVariantLayout::from_net_id(net_id);
        }
    }

    fn bytes(size: usize) -> *mut c_void {
        Box::leak(vec![0u64; size.div_ceil(8)].into_boxed_slice()).as_mut_ptr().cast()
    }

    fn table(slots: &[(u64, usize)]) -> *const usize {
        let mut table = vec![0usize; 64];
        for (slot, function) in slots {
            table[*slot as usize] = *function;
        }
        Box::leak(table.into_boxed_slice()).as_ptr()
    }

    // an object of `size` bytes whose first qword is a table with `slots`
    fn object(size: usize, slots: &[(u64, usize)]) -> *mut c_void {
        let object = Self::bytes(size);
        // SAFETY: the object has room for its vtable pointer
        unsafe { Self::poke(object, 0, Self::table(slots) as usize) };
        object
    }

    unsafe fn poke<T>(base: *mut c_void, offset: u64, value: T) {
        // SAFETY: the caller passes an offset inside the object
        unsafe { base.cast::<u8>().add(offset as usize).cast::<T>().write_unaligned(value) };
    }

    unsafe fn peek<T: Copy>(base: *const c_void, offset: u64) -> T {
        // SAFETY: the caller passes an offset inside the object
        unsafe { base.cast::<u8>().add(offset as usize).cast::<T>().read_unaligned() }
    }

    extern "C" fn local_player(this: *mut c_void) -> *mut c_void {
        // SAFETY: the fake client keeps its player at 8
        unsafe { Self::peek(this, 0x8) }
    }

    extern "C" fn attach_pos(_this: *mut c_void, out: *mut [f32; 3], location: i32, _alpha: f32) -> *mut [f32; 3] {
        assert_eq!(location, 3, "the eye is the head attach point");
        // SAFETY: the caller passes a Vec3 buffer
        unsafe { *out = [0.5, 65.62, 0.5] };
        out
    }

    extern "C" fn view_vector(_this: *mut c_void, out: *mut [f32; 3], _alpha: f32) -> *mut [f32; 3] {
        // SAFETY: the caller passes a Vec3 buffer
        unsafe { *out = [0.0, -0.5, -1.0] };
        out
    }

    extern "C" fn max_pick_range(_this: *mut c_void) -> f32 {
        12.0
    }

    extern "C" fn game_type(_this: *mut c_void) -> i32 {
        1
    }

    extern "C" fn runtime_id(_this: *mut c_void, out: *mut u64) -> *mut u64 {
        // SAFETY: the caller passes an 8-byte buffer
        unsafe { *out = 77 };
        out
    }

    extern "C" fn hit_result(this: *mut c_void) -> *mut c_void {
        // SAFETY: the fake level keeps its hit result at 8
        unsafe { Self::peek(this, 0x8) }
    }

    extern "C" fn item_registry(_this: *mut c_void, out: *mut [usize; 2]) -> *mut [usize; 2] {
        // SAFETY: the caller passes a buffer for an ItemRegistryRef
        unsafe { *out = [0, 0] };
        out
    }

    extern "C" fn creative_registry(_this: *mut c_void, out: *mut [usize; 3]) -> *mut [usize; 3] {
        // SAFETY: the caller passes a buffer for a NonOwnerPointer
        unsafe { *out = [0, 0, Self::registry()] };
        out
    }

    extern "C" fn max_stack_size(_item: *const c_void) -> u8 {
        64
    }

    extern "C" fn get_block(_region: *mut c_void, pos: *const BlockPos) -> *const c_void {
        // SAFETY: the caller passes a live position
        let pos = unsafe { *pos };
        let blocks = *BLOCKS.lock().unwrap();
        let stone = *STONE_CELL.lock().unwrap() == Some([pos.x, pos.y, pos.z]);
        blocks[usize::from(stone)] as *const c_void
    }

    extern "C" fn can_be_built_over(
        _this: *const c_void,
        block: *const c_void,
        _region: *mut c_void,
        _pos: *const BlockPos,
    ) -> u8 {
        u8::from(block as usize == BLOCKS.lock().unwrap()[0])
    }

    extern "C" fn is_interactive(_this: *const c_void) -> u8 {
        0
    }

    extern "C" fn as_item_instance(
        _this: *const c_void,
        out: *mut ItemInstanceLayout,
        _block: *const c_void,
        _actor: *const c_void,
    ) -> *mut ItemInstanceLayout {
        // SAFETY: the caller passes a zeroed item instance; mItem is at 8
        unsafe { Self::poke(out.cast(), 0x08, Self::planks()) };
        out
    }

    extern "C" fn item_dtor(_this: *mut c_void) {
        ITEM_DTOR_CALLS.fetch_add(1, Ordering::SeqCst);
    }

    extern "C" fn stack_ctor(this: *mut c_void) -> *mut c_void {
        // SAFETY: the caller passes 152 bytes for a stack
        unsafe { Self::poke(this, 0, STACK_VTABLE.load(Ordering::SeqCst)) };
        this
    }

    extern "C" fn stack_copy(this: *mut c_void, source: *const c_void) -> *mut c_void {
        // SAFETY: the caller passes 152 bytes and a live source stack
        unsafe { std::ptr::copy_nonoverlapping(source.cast::<u8>(), this.cast::<u8>(), size_of::<ItemStackLayout>()) };
        Self::push("copy_stack".to_string());
        this
    }

    extern "C" fn stack_dtor(this: *mut c_void, _flags: u32) -> *mut c_void {
        STACK_DTOR_CALLS.fetch_add(1, Ordering::SeqCst);
        this
    }

    extern "C" fn reinit(this: *mut c_void, item: *const c_void, count: i32, _aux: i32) {
        let counter = [
            STONE_COUNTER.load(Ordering::SeqCst),
            DIRT_COUNTER.load(Ordering::SeqCst),
            PLANKS_COUNTER.load(Ordering::SeqCst),
        ]
        .into_iter()
        .find(|counter| Self::item(*counter) == item)
        .unwrap_or(0);
        // SAFETY: a stack the fake constructor made
        unsafe { Self::write_stack(this.cast(), counter, count as u8, 0) };
    }

    extern "C" fn set_item(this: *mut c_void, slot: i32, stack: *const ItemStackLayout) {
        Self::push(format!("set_item {slot}"));
        // SAFETY: the fake container and a live stack
        unsafe {
            let items = Self::peek::<usize>(this, Self::ITEMS) as *mut ItemStackLayout;
            let item = Self::peek::<usize>(stack.cast(), 0x08);
            Self::write_stack(items.add(slot as usize), item, (*stack).base.count, 0);
        }
    }

    extern "C" fn swap_slots(this: *mut c_void, a: i32, b: i32) {
        Self::push(format!("swap_slots {a} {b}"));
        // SAFETY: the fake container holds 36 stacks
        unsafe {
            let items = Self::peek::<usize>(this, Self::ITEMS) as *mut ItemStackLayout;
            std::ptr::swap(items.add(a as usize), items.add(b as usize));
        }
    }

    extern "C" fn handle_slot_selection(_this: *mut c_void, slot: i32, container: i32) {
        assert_eq!(container, 0, "the select names the inventory container");
        Self::push(format!("select {slot}"));
        if SELECT_TAKES.load(Ordering::SeqCst) {
            // SAFETY: the fake PlayerInventory holds mSelected at SELECTED
            unsafe { Self::poke(SUPPLIES.load(Ordering::SeqCst) as *mut c_void, Self::SELECTED, slot) };
        }
    }

    extern "C" fn build_block(this: *mut c_void, pos: *const BlockPos, face: u8, hand: u8, sim: u8) -> u8 {
        let build = || {
            // SAFETY: the caller passes a live position
            let pos = unsafe { *pos };
            Self::push(format!("build {} {} {} face={face} hand={hand} sim={sim}", pos.x, pos.y, pos.z));
            u8::from(BUILD_OK.load(Ordering::SeqCst))
        };
        let hooks = BUILD_HOOKS.load(Ordering::SeqCst);
        if hooks == 0 {
            return build();
        }
        // SAFETY: the hooks the test stored, leaked for the test process
        let hooks = unsafe { &*(hooks as *const UseHooks) };
        alaydriem_stratum::detours::HookBody::skip_or(build, 0, &LATCH, || {
            // SAFETY: a fake game mode laid out as the calls read it
            let skip = unsafe { hooks.blocks_game_build(this, sim != 0) };
            if skip {
                Self::push("build skipped".to_string());
            }
            skip
        })
    }
}
