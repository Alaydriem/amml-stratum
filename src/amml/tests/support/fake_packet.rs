use std::ffi::c_void;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, AtomicUsize, Ordering};

use alaydriem_stratum::types::{PacketCalls, PlayerCalls};
use amml_game::ReadOnlyBinaryStreamLayout;
use amml_msvc::{RefCountBase, RefCountVtable};

/// Fake game objects for the packet, stream, sender and item stack wrappers, with counters. Each test runs in its own
/// process (nextest), so the counters start at zero.
pub struct FakePacket;

/// The first qword the fake base-only stack constructor writes.
pub const BASE_STACK_MARK: usize = 0xBA5E;
pub static FULL_STACK_VTABLE: AtomicUsize = AtomicUsize::new(0);
pub static FULL_STACK_BYTES: Mutex<Vec<u8>> = Mutex::new(Vec::new());
pub static CREATE_CALLS: AtomicU32 = AtomicU32::new(0);
pub static DESTROY_CALLS: AtomicU32 = AtomicU32::new(0);
pub static DELETE_CALLS: AtomicU32 = AtomicU32::new(0);
pub static READ_CALLS: AtomicU32 = AtomicU32::new(0);
/// Whether the fake `Packet::read` succeeds.
pub static READ_OK: AtomicBool = AtomicBool::new(true);
/// The bytes the fake `Packet::read` saw through the stream's view.
pub static READ_BYTES: Mutex<Vec<u8>> = Mutex::new(Vec::new());
pub static STREAM_DTOR_CALLS: AtomicU32 = AtomicU32::new(0);
pub static SEND_CALLS: AtomicU32 = AtomicU32::new(0);
/// The packet the fake `PacketSender::send` got last.
pub static SENT: Mutex<usize> = Mutex::new(0);
pub static STACK_DTOR_CALLS: AtomicU32 = AtomicU32::new(0);
/// The flags the fake ItemStack deleting destructor got last.
pub static STACK_DTOR_FLAGS: AtomicU32 = AtomicU32::new(u32::MAX);
pub static STACK_COPY_CALLS: AtomicU32 = AtomicU32::new(0);
/// The name the fake packet gives through `getName`.
pub static PACKET_NAME: Mutex<&'static str> = Mutex::new("ItemStackRequestPacket");
/// When set, each fake packet gives the name of the id the factory made it for, as the game's packets do.
pub static NAME_BY_ID: AtomicBool = AtomicBool::new(false);
/// The ids of the packets the fake `PacketSender::send` got, in order.
pub static SENT_IDS: Mutex<Vec<u32>> = Mutex::new(Vec::new());

/// The vtable slot of `Packet::read(ReadOnlyBinaryStream&)` in the fake packet.
pub const READ_SLOT: u64 = 8;
/// The vtable slot of `ClientInstance::getPacketSender` in the fake client.
pub const SENDER_SLOT: u64 = 3;
/// The vtable slot of `PacketSender::send` in the fake sender.
pub const SEND_SLOT: u64 = 2;
/// The fake stream vtable: an address the fake dtor checks.
pub static STREAM_VTABLE: [usize; 2] = [0, 0];

/// A fake packet object: its vtable pointer, room for a payload, and the id the factory made it for.
#[repr(C)]
pub struct PacketObject {
    pub vtable: *const usize,
    pub payload: [u8; 0x60],
    pub id: u32,
}

/// A fake object with only a vtable pointer: the client, the packet sender and an item stack.
#[repr(C)]
pub struct VtableObject {
    pub vtable: *const usize,
}

static CONTROL_VTABLE: RefCountVtable = RefCountVtable {
    destroy: FakePacket::destroy,
    delete_this: FakePacket::delete_this,
    destructor: FakePacket::control_destructor,
    get_deleter: FakePacket::get_deleter,
};

#[allow(unsafe_code)]
impl FakePacket {
    pub fn calls() -> PacketCalls {
        PacketCalls {
            create_packet: Self::create as *const () as usize,
            packet_read_slot: READ_SLOT,
            stream_vtable: STREAM_VTABLE.as_ptr() as usize,
            stream_dtor: Self::stream_dtor as *const () as usize,
            get_packet_sender_slot: SENDER_SLOT,
            send_slot: SEND_SLOT,
        }
    }

    /// Player calls with the fake ItemStack copy constructor and `stack_vtable` as the ItemStack vtable.
    pub fn player_calls(stack_vtable: *const usize) -> PlayerCalls {
        PlayerCalls {
            player_inventory: 0,
            swap_slots_slot: 44,
            set_item_slot: 12,
            item_stack_empty_ctor: Self::stack_ctor as *const () as usize,
            item_stack_copy_ctor: Self::stack_copy as *const () as usize,
            item_stack_vtable: stack_vtable as usize,
            item_stack_reinit_slot: 3,
            get_max_stack_size: 0,
            actor_level: 0,
            level_get_item_registry_slot: 0,
            get_creative_item_registry: 0,
            creative_items: 0x18,
            get_runtime_id: 0,
            get_player_game_type: 0,
        }
    }

    pub fn count(counter: &AtomicU32) -> u32 {
        counter.load(Ordering::SeqCst)
    }

    /// The vtable of the fake ItemStack: the deleting destructor at 0.
    pub fn stack_vtable() -> &'static [usize; 7] {
        let mut table = [0usize; 7];
        table[0] = Self::stack_deleting_dtor as *const () as usize;
        Box::leak(Box::new(table))
    }

    /// The fake sender the fake client gives.
    pub fn sender() -> *mut c_void {
        static SENDER: Mutex<usize> = Mutex::new(0);
        let mut sender = SENDER.lock().unwrap();
        if *sender == 0 {
            let mut table = [0usize; 3];
            table[SEND_SLOT as usize] = Self::send as *const () as usize;
            let table = Box::leak(Box::new(table));
            *sender = Box::leak(Box::new(VtableObject { vtable: table.as_ptr() })) as *mut VtableObject as usize;
        }
        *sender as *mut c_void
    }

    /// A fake client whose getPacketSender gives `sender()`.
    pub fn client() -> *mut c_void {
        let mut table = [0usize; 4];
        table[SENDER_SLOT as usize] = Self::get_packet_sender as *const () as usize;
        let table = Box::leak(Box::new(table));
        Box::leak(Box::new(VtableObject { vtable: table.as_ptr() })) as *mut VtableObject as *mut c_void
    }

    // a new fake packet with one use and its control block; both leak for the test process
    fn leak_packet(id: u32) -> (*mut PacketObject, *mut RefCountBase) {
        let mut table = [0usize; 10];
        table[2] = Self::get_name as *const () as usize;
        table[READ_SLOT as usize] = Self::read as *const () as usize;
        let table = Box::leak(Box::new(table));
        let packet = Box::leak(Box::new(PacketObject { vtable: table.as_ptr(), payload: [0; 0x60], id }));
        let control = Box::leak(Box::new(RefCountBase {
            vtable: &CONTROL_VTABLE,
            uses: AtomicI32::new(1),
            weaks: AtomicI32::new(1),
        }));
        (packet, control)
    }

    // std::shared_ptr<Packet> MinecraftPackets::createPacket(MinecraftPacketIds): a new fake packet with one use
    extern "C" fn create(out: *mut [usize; 2], id: u32) -> *mut [usize; 2] {
        CREATE_CALLS.fetch_add(1, Ordering::SeqCst);
        let (packet, control) = Self::leak_packet(id);
        // SAFETY: the wrapper passes a buffer for a shared_ptr
        unsafe { *out = [packet as usize, control as usize] };
        out
    }

    extern "C" fn get_name(this: *const c_void, out: *mut [usize; 2]) -> *mut [usize; 2] {
        // SAFETY: the wrapper passes a fake packet this factory made
        let id = unsafe { (*this.cast::<PacketObject>()).id };
        let name = if NAME_BY_ID.load(Ordering::SeqCst) {
            match id {
                33 => "InteractPacket",
                46 => "ContainerOpenPacket",
                47 => "ContainerClosePacket",
                147 => "ItemStackRequestPacket",
                _ => "UnknownPacket",
            }
        } else {
            *PACKET_NAME.lock().unwrap()
        };
        // SAFETY: the wrapper passes a buffer for a string_view
        unsafe { *out = [name.as_ptr() as usize, name.len()] };
        out
    }

    // Bedrock::Result<void> Packet::read(ReadOnlyBinaryStream&): this, the result, the stream
    extern "C" fn read(_this: *mut c_void, result: *mut u8, stream: *mut ReadOnlyBinaryStreamLayout) -> *mut u8 {
        READ_CALLS.fetch_add(1, Ordering::SeqCst);
        let ok = READ_OK.load(Ordering::SeqCst);
        // SAFETY: the wrapper passes a live stream over live bytes and a result buffer of at least 0x48 bytes
        unsafe {
            let stream = &mut *stream;
            *READ_BYTES.lock().unwrap() = std::slice::from_raw_parts(stream.view, stream.view_len).to_vec();
            if ok {
                stream.read_pointer = stream.view_len as u64;
            }
            result.add(0x40).write(u8::from(ok));
        }
        result
    }

    extern "C" fn stream_dtor(this: *mut ReadOnlyBinaryStreamLayout) {
        STREAM_DTOR_CALLS.fetch_add(1, Ordering::SeqCst);
        // SAFETY: the wrapper passes its live stream
        let vtable = unsafe { (*this).vtable };
        assert_eq!(vtable as usize, STREAM_VTABLE.as_ptr() as usize, "the stream was made with the stream vtable");
    }

    pub extern "C" fn get_packet_sender(_this: *mut c_void) -> *mut c_void {
        Self::sender()
    }

    extern "C" fn send(_this: *mut c_void, packet: *mut c_void) {
        SEND_CALLS.fetch_add(1, Ordering::SeqCst);
        *SENT.lock().unwrap() = packet as usize;
        // SAFETY: the wrapper sends a fake packet this factory made
        SENT_IDS.lock().unwrap().push(unsafe { (*packet.cast::<PacketObject>()).id });
    }

    extern "C" fn stack_ctor(this: *mut c_void) -> *mut c_void {
        this
    }

    /// The ItemStack table the fake 26.51 constructor stores.
    pub fn set_full_stack_vtable(table: usize) {
        FULL_STACK_VTABLE.store(table, Ordering::SeqCst);
    }

    /// The 152 bytes the fake 26.51 constructor left in the stack.
    pub fn full_stack_bytes() -> Vec<u8> {
        FULL_STACK_BYTES.lock().unwrap().clone()
    }

    /// A constructor that writes what the 26.51 `ItemStack::ItemStack()` writes at the end (the ItemStack vtable at
    /// +0, `mov dword ptr [rax+80h],0; mov byte ptr [rax+90h],0`), with marks in the base part, and keeps a copy.
    pub extern "C" fn full_stack_ctor(this: *mut c_void) -> *mut c_void {
        // SAFETY: the caller passes 152 zeroed bytes for a stack
        unsafe {
            let bytes = this.cast::<u8>();
            bytes.add(0x23).write(1);
            bytes.add(0x24).write(1);
            this.cast::<usize>().write(FULL_STACK_VTABLE.load(Ordering::SeqCst));
            bytes.add(0x80).cast::<u32>().write_unaligned(0);
            bytes.add(0x90).write(0);
            *FULL_STACK_BYTES.lock().unwrap() = std::slice::from_raw_parts(bytes, 0x98).to_vec();
        }
        this
    }

    /// A base-only constructor that also leaves junk in the net id variant, to show the after-step stores.
    pub extern "C" fn dirty_base_stack_ctor(this: *mut c_void) -> *mut c_void {
        // SAFETY: the caller passes 152 bytes for a stack
        unsafe {
            this.cast::<usize>().write(BASE_STACK_MARK);
            this.cast::<u8>().add(0x80).write_bytes(0xEE, 0x18);
        }
        this
    }

    /// A constructor that builds only the `ItemStackBase` part, as 26.60 does: it stores a base table mark at +0.
    pub extern "C" fn base_stack_ctor(this: *mut c_void) -> *mut c_void {
        // SAFETY: the caller passes 152 bytes for a stack
        unsafe { this.cast::<usize>().write(BASE_STACK_MARK) };
        this
    }

    extern "C" fn stack_copy(this: *mut c_void, _source: *const c_void) -> *mut c_void {
        STACK_COPY_CALLS.fetch_add(1, Ordering::SeqCst);
        this
    }

    extern "C" fn stack_deleting_dtor(this: *mut c_void, flags: u32) -> *mut c_void {
        STACK_DTOR_CALLS.fetch_add(1, Ordering::SeqCst);
        STACK_DTOR_FLAGS.store(flags, Ordering::SeqCst);
        this
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
