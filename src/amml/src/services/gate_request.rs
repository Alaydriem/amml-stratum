use std::ffi::c_void;

use amml_game::{
    ContainerClosePacketLayout, ContainerOpenPacketLayout, InteractPacketLayout, ItemStackResponseInfoLayout,
    ItemStackResponsePacketLayout,
};
use amml_msvc::MemberCall;

use crate::easy_place::{RequestEncoder, SlotRef};
use crate::game::{CreativeItems, GamePacket, ItemName, OwnedItemStack, PacketId, PacketSender, PlayerInventory};
use crate::types::{PacketCalls, PlayerCalls};

/// With the `gate` feature, on the game thread: some seconds after the first world tick, sends two real item stack
/// requests, as the game's own inventory screen does: first a creative pick that fills hotbar slot 1 with the first
/// creative item that is not the item in slot 0, then a swap of slots 0 and 1. For each request: the request is read
/// into its packet, the inventory is opened (Interact OpenInventory), the request is sent and predicted, the server's
/// open of the inventory is hidden and answered with a close, the server's answer to that close is hidden, and the
/// ItemStackResponse with the request's id ends it. A refused or unanswered request undoes its prediction and ends
/// the gate. Each step logs one line. Once per enable.
pub struct GateRequest {
    packets: PacketCalls,
    players: PlayerCalls,
    first_world: Option<f64>,
    slots_logged: bool,
    stage: u8,
    request_id: i32,
    sent_at: f64,
    picked_at: f64,
    open_until: f64,
    close_until: f64,
    opened_id: Option<i8>,
    // what the pick replaced in slot 1, put back when the pick is refused
    undo_pick: Option<OwnedItemStack>,
    done: bool,
}

// SAFETY: the calls are plain addresses and the undo stack is the mod's own memory; the gate runs on the game thread
// only, under the runtime's lock
#[allow(unsafe_code)]
unsafe impl Send for GateRequest {}

impl GateRequest {
    /// Seconds after the first world tick at which the gate logs the two hotbar slots and the game mode.
    pub const SLOTS_AT: f64 = 5.0;
    /// Seconds after the first world tick at which the gate sends the pick.
    pub const SEND_AT: f64 = 10.0;
    /// Seconds between the pick's response and the swap, so the pick's close answer has come.
    pub const SWAP_AFTER: f64 = 2.0;
    /// Seconds the gate waits for the server's open and its close answer.
    pub const ANSWER_WINDOW: f64 = 2.0;
    pub const RESPONSE_TIMEOUT: f64 = 5.0;
    /// The request ids: negative and odd, as the game's own client ids are, and as the C++ started them.
    pub const PICK_ID: i32 = -100001;
    pub const SWAP_ID: i32 = -100003;
    const IDLE: u8 = 0;
    const PICK_SENT: u8 = 1;
    const PICKED: u8 = 2;
    const SWAP_SENT: u8 = 3;
    // InteractPacketPayload::Action::OpenInventory
    const OPEN_INVENTORY: u8 = 6;
    // SharedTypes::Legacy::ContainerType: Inventory -1, None -9
    const INVENTORY_TYPE: i8 = -1;
    const NO_TYPE: i8 = -9;
    // GameType::Creative
    const CREATIVE: i32 = 1;

    pub fn new(packets: PacketCalls, players: PlayerCalls) -> Self {
        Self {
            packets,
            players,
            first_world: None,
            slots_logged: false,
            stage: Self::IDLE,
            request_id: 0,
            sent_at: 0.0,
            picked_at: 0.0,
            open_until: 0.0,
            close_until: 0.0,
            opened_id: None,
            undo_pick: None,
            done: false,
        }
    }

    /// Whether the gate has ended: both responses came, a request was refused or timed out, or it was not sent.
    pub fn settled(&self) -> bool {
        self.done
    }

    /// Once per game tick while a local player is in a world: `client` the live `ClientInstance`, `player` its live
    /// local player, `now` the steady clock in seconds.
    ///
    /// # Safety
    /// `client` and `player` must be live objects of the build the calls were resolved for.
    #[allow(unsafe_code)]
    pub unsafe fn tick(&mut self, now: f64, client: *mut c_void, player: *mut c_void, log: &mut impl FnMut(&str)) {
        if self.done {
            return;
        }
        let start = *self.first_world.get_or_insert(now);
        // SAFETY: the caller guarantees a live local player
        let Some(inventory) = (unsafe { PlayerInventory::of_player(player, &self.players) }) else { return };
        if !self.slots_logged && now - start >= Self::SLOTS_AT {
            self.slots_logged = true;
            // SAFETY: as above
            let mode = unsafe { self.game_type(player) };
            log(&format!("gate slots mode={} {}", Self::mode_name(mode), Self::slots(&inventory)));
        }
        match self.stage {
            Self::IDLE if now - start >= Self::SEND_AT => {
                // SAFETY: as above, for the client too
                unsafe { self.send_pick(now, client, player, &inventory, log) };
            }
            Self::PICKED if now - self.picked_at >= Self::SWAP_AFTER => {
                // SAFETY: as above
                unsafe { self.send_swap(now, client, player, &inventory, log) };
            }
            Self::PICK_SENT | Self::SWAP_SENT if now - self.sent_at >= Self::RESPONSE_TIMEOUT => {
                log(&format!("gate response id={} result=timeout", self.request_id));
                self.undo(&inventory);
                self.done = true;
            }
            _ => {}
        }
    }

    /// The server's container open: true hides it from the game, when it is the inventory open the gate asked for;
    /// the gate then sends the close for it.
    ///
    /// # Safety
    /// `client` must be null or the live `ClientInstance`.
    #[allow(unsafe_code)]
    pub unsafe fn on_container_open(
        &mut self,
        now: f64,
        packet: &ContainerOpenPacketLayout,
        client: *mut c_void,
        log: &mut impl FnMut(&str),
    ) -> bool {
        if now > self.open_until || packet.container_type != Self::INVENTORY_TYPE {
            return false;
        }
        self.open_until = 0.0;
        self.opened_id = Some(packet.container_id);
        // SAFETY: the caller guarantees a null or live client
        let closed = unsafe { self.close_inventory(client, packet.container_id) };
        if closed {
            self.close_until = now + Self::ANSWER_WINDOW;
        }
        log(&format!("gate container open id={} hidden close_sent={closed}", packet.container_id));
        true
    }

    /// The server's container close: true hides it from the game, when it answers the close the gate sent.
    pub fn on_container_close(
        &mut self,
        now: f64,
        packet: &ContainerClosePacketLayout,
        log: &mut impl FnMut(&str),
    ) -> bool {
        if now > self.close_until || Some(packet.container_id) != self.opened_id {
            return false;
        }
        self.close_until = 0.0;
        log(&format!("gate container close id={} hidden", packet.container_id));
        true
    }

    /// The server's item stack responses: the one with the request's id ends that request. An accepted pick gives
    /// slot 1 the net id the server chose; a refusal undoes the prediction and ends the gate.
    ///
    /// # Safety
    /// `player` must be null or the live local player.
    #[allow(unsafe_code)]
    pub unsafe fn on_response(
        &mut self,
        now: f64,
        packet: &ItemStackResponsePacketLayout,
        player: *mut c_void,
        log: &mut impl FnMut(&str),
    ) {
        if self.done || self.request_id == 0 {
            return;
        }
        let Some(response) =
            packet.responses.as_slice().iter().find(|response| response.client_request_id == self.request_id)
        else {
            return;
        };
        log(&format!("gate response id={} result={}", self.request_id, response.result));
        // SAFETY: the caller guarantees a null or live local player
        let Some(inventory) = (unsafe { PlayerInventory::of_player(player, &self.players) }) else {
            self.done = true;
            return;
        };
        if response.result != 0 {
            self.undo(&inventory);
            self.done = true;
            log(&format!("gate slots after undo {}", Self::slots(&inventory)));
            return;
        }
        self.request_id = 0;
        if self.stage == Self::PICK_SENT {
            let net_id = Self::hotbar_net_id(response, 1);
            if let Some(net_id) = net_id {
                inventory.set_net_id(1, net_id);
            }
            self.undo_pick = None;
            self.stage = Self::PICKED;
            self.picked_at = now;
            log(&format!("gate slots after pick server_net={} {}", net_id.unwrap_or(0), Self::slots(&inventory)));
            return;
        }
        self.done = true;
        log(&format!("gate slots after swap {}", Self::slots(&inventory)));
    }

    // the creative pick into hotbar slot 1 of the first creative item that is not slot 0's item, with the C++
    // prediction: what slot 1 held is kept for the undo, and slot 1 gets a full stack of the item
    #[allow(unsafe_code)]
    unsafe fn send_pick(
        &mut self,
        now: f64,
        client: *mut c_void,
        player: *mut c_void,
        inventory: &PlayerInventory,
        log: &mut impl FnMut(&str),
    ) {
        // SAFETY: the caller guarantees a live local player
        let mode = unsafe { self.game_type(player) };
        if mode != Some(Self::CREATIVE) {
            log(&format!("gate request skipped: the game mode is {}, not creative", Self::mode_name(mode)));
            self.done = true;
            return;
        }
        let (Some(first), Some(held)) = (inventory.item(0), inventory.item(1)) else {
            log("gate request skipped: no hotbar slots 0 and 1");
            self.done = true;
            return;
        };
        // SAFETY: a stack of the live container; its item counter is live while the stack is
        let first_item = unsafe { first.base.item() };
        // SAFETY: as above
        let Some(creative) = (unsafe { CreativeItems::of_player(player, &self.players) }) else {
            log("gate request skipped: no creative item registry");
            self.done = true;
            return;
        };
        let entry = creative.entries().iter().find(|entry| {
            // SAFETY: an entry of the live registry; its item counter is live while the registry is
            let item = unsafe { entry.item_instance.item() };
            !item.is_null() && item != first_item
        });
        let Some(entry) = entry else {
            log("gate request skipped: no creative item other than slot 0's");
            self.done = true;
            return;
        };
        // SAFETY: as above
        let item = unsafe { entry.item_instance.item() };
        // SAFETY: the entry's live Item
        let name = unsafe { ItemName::of(item) }.unwrap_or_else(|| format!("item@{item:p}"));
        let amount = creative.max_stack_size(&entry.item_instance);
        let net_id = entry.creative_net_id as i32;
        // SAFETY: as above, for the held stack
        let held_empty = unsafe { held.base.item() }.is_null() || held.base.count == 0;
        let to = SlotRef { container: SlotRef::HOTBAR, slot: 1, net_id: held.net_id_variant.net_id().unwrap_or(0) };
        let bytes = if held_empty {
            RequestEncoder::pick(Self::PICK_ID, net_id, amount, to)
        } else {
            RequestEncoder::pick_replacing(Self::PICK_ID, net_id, amount, to, held.base.count)
        };
        // SAFETY: the caller guarantees a live client and player
        let opened = unsafe { self.send_request(now, client, player, Self::PICK_ID, &bytes, log) };
        let Some(opened) = opened else { return };
        // SAFETY: the live held stack and the live item of the creative entry
        unsafe {
            self.undo_pick = Some(OwnedItemStack::copy_of(held, &self.players));
            let mut predicted = OwnedItemStack::new(&self.players);
            predicted.reinit(item, i32::from(amount), i32::from(entry.item_instance.aux_value), &self.players);
            inventory.set_item(1, &predicted);
        }
        self.stage = Self::PICK_SENT;
        log(&format!(
            "gate request id={} kind=pick sent=true open_sent={opened} item={name} creative_net={net_id} \
             count={amount} replaced={}",
            Self::PICK_ID,
            !held_empty
        ));
    }

    // the swap of hotbar slots 0 and 1, with the C++ prediction: the game's own swapSlots
    #[allow(unsafe_code)]
    unsafe fn send_swap(
        &mut self,
        now: f64,
        client: *mut c_void,
        player: *mut c_void,
        inventory: &PlayerInventory,
        log: &mut impl FnMut(&str),
    ) {
        let (Some(first), Some(second)) = (inventory.item(0), inventory.item(1)) else {
            log("gate request skipped: no hotbar slots 0 and 1");
            self.done = true;
            return;
        };
        // SAFETY: the stacks of the live container; their item counters are live while the stacks are
        let items = unsafe { (first.base.item(), second.base.item()) };
        let empty = items.0.is_null() || items.1.is_null() || first.base.count == 0 || second.base.count == 0;
        if empty || items.0 == items.1 {
            log("gate request skipped: hotbar slots 0 and 1 need two different items");
            self.done = true;
            return;
        }
        let from = SlotRef { container: SlotRef::HOTBAR, slot: 0, net_id: first.net_id_variant.net_id().unwrap_or(0) };
        let to = SlotRef { container: SlotRef::HOTBAR, slot: 1, net_id: second.net_id_variant.net_id().unwrap_or(0) };
        let bytes = RequestEncoder::swap(Self::SWAP_ID, from, to);
        // SAFETY: the caller guarantees a live client and player
        let opened = unsafe { self.send_request(now, client, player, Self::SWAP_ID, &bytes, log) };
        let Some(opened) = opened else { return };
        inventory.swap_slots(0, 1);
        self.stage = Self::SWAP_SENT;
        log(&format!(
            "gate request id={} kind=swap sent=true open_sent={opened} net {} {}",
            Self::SWAP_ID,
            from.net_id,
            to.net_id
        ));
    }

    // reads `bytes` into a request packet, opens the inventory and sends the request; Some(whether the open was
    // sent) once sent. A request that is not sent is logged and ends the gate
    #[allow(unsafe_code)]
    unsafe fn send_request(
        &mut self,
        now: f64,
        client: *mut c_void,
        player: *mut c_void,
        id: i32,
        bytes: &[u8],
        log: &mut impl FnMut(&str),
    ) -> Option<bool> {
        // SAFETY: the caller guarantees a live client
        let Some(sender) = (unsafe { PacketSender::of_client(client, &self.packets) }) else {
            log(&format!("gate request id={id} sent=false (no packet sender)"));
            self.done = true;
            return None;
        };
        // SAFETY: the caller guarantees a live player
        let runtime_id = unsafe { self.runtime_id(player) };
        let packets = self.packets;
        let mut opened = false;
        let sent = sender.send_bytes(&self.packets, PacketId::ITEM_STACK_REQUEST, bytes, |sender| {
            opened = Self::open_inventory(&packets, sender, runtime_id);
        });
        if let Err(error) = sent {
            log(&format!("gate request id={id} sent=false ({error})"));
            self.done = true;
            return None;
        }
        self.request_id = id;
        self.sent_at = now;
        if opened {
            self.open_until = now + Self::ANSWER_WINDOW;
        }
        Some(opened)
    }

    // puts back what the prediction of the request in flight changed
    fn undo(&mut self, inventory: &PlayerInventory) {
        match self.stage {
            Self::PICK_SENT => {
                if let Some(held) = self.undo_pick.take() {
                    inventory.set_item(1, &held);
                }
            }
            Self::SWAP_SENT => inventory.swap_slots(0, 1),
            _ => {}
        }
    }

    // the net id the server gave hotbar slot `slot` in an accepted response
    fn hotbar_net_id(response: &ItemStackResponseInfoLayout, slot: u8) -> Option<i32> {
        response
            .containers
            .as_slice()
            .iter()
            .filter(|container| container.container_name == SlotRef::HOTBAR)
            .flat_map(|container| container.slots.as_slice())
            .find(|info| info.slot == slot)
            .map(|info| info.item_stack_net_id)
    }

    // the Interact OpenInventory packet for the player, as the game's own inventory key sends it
    fn open_inventory(calls: &PacketCalls, sender: &PacketSender, runtime_id: Option<u64>) -> bool {
        let Some(target) = runtime_id else { return false };
        let Some(mut packet) = GamePacket::create(calls, PacketId::INTERACT) else { return false };
        let Some(interact) = packet.payload_mut::<InteractPacketLayout>() else { return false };
        interact.action = Self::OPEN_INVENTORY;
        interact.target_id = target;
        sender.send(&packet);
        true
    }

    // the ContainerClose for the inventory the server opened, as a client close (not server-initiated)
    #[allow(unsafe_code)]
    unsafe fn close_inventory(&self, client: *mut c_void, container_id: i8) -> bool {
        // SAFETY: the caller guarantees a null or live client
        let Some(sender) = (unsafe { PacketSender::of_client(client, &self.packets) }) else { return false };
        let Some(mut packet) = GamePacket::create(&self.packets, PacketId::CONTAINER_CLOSE) else { return false };
        let Some(close) = packet.payload_mut::<ContainerClosePacketLayout>() else { return false };
        close.container_id = container_id;
        close.container_type = Self::NO_TYPE;
        close.server_initiated_close = false;
        sender.send(&packet);
        true
    }

    // `GameType Player::getPlayerGameType() const`
    #[allow(unsafe_code)]
    unsafe fn game_type(&self, player: *mut c_void) -> Option<i32> {
        if self.players.get_player_game_type == 0 || player.is_null() {
            return None;
        }
        // SAFETY: the entry is the address of getPlayerGameType, which returns the enum in eax
        let call: unsafe extern "C" fn(*mut c_void) -> i32 =
            unsafe { std::mem::transmute(self.players.get_player_game_type) };
        // SAFETY: the caller guarantees a live player
        Some(unsafe { call(player) })
    }

    // `ActorRuntimeID Actor::getRuntimeID() const`, through the hidden return pointer
    #[allow(unsafe_code)]
    unsafe fn runtime_id(&self, player: *mut c_void) -> Option<u64> {
        if self.players.get_runtime_id == 0 || player.is_null() {
            return None;
        }
        // SAFETY: the entry is the address of getRuntimeID, a member function that returns an 8-byte class
        Some(unsafe { MemberCall::call_sret0::<u64>(self.players.get_runtime_id as *const c_void, player) })
    }

    fn mode_name(mode: Option<i32>) -> String {
        match mode {
            Some(0) => "survival".to_string(),
            Some(1) => "creative".to_string(),
            Some(2) => "adventure".to_string(),
            Some(5) => "default".to_string(),
            Some(6) => "spectator".to_string(),
            Some(other) => other.to_string(),
            None => "unknown".to_string(),
        }
    }

    // hotbar slots 0 and 1 for the log
    fn slots(inventory: &PlayerInventory) -> String {
        format!("0={} 1={}", Self::slot(inventory, 0), Self::slot(inventory, 1))
    }

    // one slot for the log: the item's name, the count and the net id
    #[allow(unsafe_code)]
    fn slot(inventory: &PlayerInventory, slot: usize) -> String {
        let Some(stack) = inventory.item(slot) else { return "none".to_string() };
        // SAFETY: a stack of the live container
        let item = unsafe { stack.base.item() };
        if item.is_null() || stack.base.count == 0 {
            return "empty".to_string();
        }
        // SAFETY: the stack's live Item
        let name = unsafe { ItemName::of(item) }.unwrap_or_else(|| format!("item@{item:p}"));
        format!("{name}x{} net={}", stack.base.count, stack.net_id_variant.net_id().unwrap_or(0))
    }
}
