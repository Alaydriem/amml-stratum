use std::ffi::c_void;

use amml_game::{
    ContainerClosePacketLayout, ContainerOpenPacketLayout, CreativeItemEntryLayout, InteractPacketLayout,
    ItemStackLayout, ItemStackResponseInfoLayout, ItemStackResponsePacketLayout, RawObject,
};
use amml_msvc::MemberCall;
use stratum_core::structs::easy_place::SwapAnswer;

use crate::easy_place::{RequestEncoder, SlotRef};
use crate::errors::PacketReadError;
use crate::game::{CreativeItems, GamePacket, OwnedItemStack, PacketId, PacketSender, PlayerInventory, WorldBlocks};
use crate::types::{PacketCalls, PlayerCalls, UseCalls};

/// Sends the item stack requests that move a stack into the hotbar, the way the game's own inventory screen does: a
/// hidden inventory open, one request, a client prediction, then a close once the server opens its side. The server's
/// open and its answer to the close are hidden from the game, and the answer to the request is kept for the next use
/// call. The prediction is undone when the server refuses the request, when no answer comes in time, and when a new
/// request replaces an unanswered one, each time only while the predicted slot still holds the predicted item. Holds
/// one request in flight at a time.
pub struct StackRequest {
    packets: PacketCalls,
    players: PlayerCalls,
    calls: UseCalls,
    blocks: WorldBlocks,
    // the client the last request was sent through
    client: *mut c_void,
    next_id: i32,
    await_id: i32,
    sent_at: f64,
    answer: SwapAnswer,
    open_until: f64,
    close_until: f64,
    opened_id: i32,
    // the prediction of the request in flight: what it changed, the hotbar slot, the main inventory slot a swap took
    // from, the item the hotbar slot holds after the prediction, and the stack a pick replaced
    undo_kind: u8,
    undo_slot: i32,
    undo_from: i32,
    undo_item: *const c_void,
    undo_stack: Option<OwnedItemStack>,
}

// SAFETY: the raw pointers are game objects used on the game thread only, under the runtime's lock; another thread
// only moves the value or drops it at disable
#[allow(unsafe_code)]
unsafe impl Send for StackRequest {}

impl StackRequest {
    // how long the server's open and close answers are waited for
    const ANSWER_WINDOW_SECONDS: f64 = 2.0;
    /// The wait for an answer: the same as core's default EasyPlaceConfig swap_timeout_ms.
    pub const REQUEST_TIMEOUT_SECONDS: f64 = 0.5;
    /// Request ids are negative and odd, the way the game's own client ids are; a step of 2 keeps them odd.
    pub const FIRST_REQUEST_ID: i32 = -100001;
    const REQUEST_ID_STEP: i32 = 2;
    const UNDO_NONE: u8 = 0;
    const UNDO_SWAP: u8 = 1;
    const UNDO_PICK: u8 = 2;
    const HOTBAR_SLOTS: i32 = 9;
    const INVENTORY_SLOTS: i32 = 36;
    // InteractPacketPayload::Action::OpenInventory
    const OPEN_INVENTORY: u8 = 6;
    // SharedTypes::Legacy::ContainerType: Inventory -1, None -9
    const INVENTORY_TYPE: i8 = -1;
    const NO_TYPE: i8 = -9;

    pub fn new(packets: &PacketCalls, players: &PlayerCalls, calls: &UseCalls) -> Self {
        Self {
            packets: *packets,
            players: *players,
            calls: *calls,
            blocks: WorldBlocks::new(calls),
            client: std::ptr::null_mut(),
            next_id: Self::FIRST_REQUEST_ID,
            await_id: 0,
            sent_at: 0.0,
            answer: SwapAnswer::None,
            open_until: 0.0,
            close_until: 0.0,
            opened_id: -1,
            undo_kind: Self::UNDO_NONE,
            undo_slot: -1,
            undo_from: -1,
            undo_item: std::ptr::null(),
            undo_stack: None,
        }
    }

    /// The answer to the last request, as the probe's swap answer reads it.
    pub fn answer(&self) -> SwapAnswer {
        self.answer
    }

    /// The id of the request in flight; 0 when none is.
    pub fn awaited(&self) -> i32 {
        self.await_id
    }

    /// Exchanges main inventory slot `from` (9 to 35) with hotbar slot `to` (0 to 8). A request that cannot be written
    /// or sent is answered as refused at once.
    ///
    /// # Safety
    /// `client` and `player` must be the live client and its live local player.
    #[allow(unsafe_code)]
    pub unsafe fn swap(&mut self, now: f64, client: *mut c_void, player: *mut c_void, from: i32, to: i32) {
        if self.await_id != 0 {
            self.finish(true);
        }
        self.answer = SwapAnswer::Refused;
        // SAFETY: the caller guarantees a live player
        let inventory = unsafe { PlayerInventory::of_player(player, &self.players) };
        let sides = inventory.as_ref().and_then(|items| Some((items.item(from as usize)?, items.item(to as usize)?)));
        let in_range =
            (0..Self::HOTBAR_SLOTS).contains(&to) && (Self::HOTBAR_SLOTS..Self::INVENTORY_SLOTS).contains(&from);
        let (Some(inventory), Some((source, target)), true) = (inventory.as_ref(), sides, in_range) else {
            curia::warn!("easy place swap not sent", { from: from, to: to });
            return;
        };
        let empty = Self::is_empty(target);
        // SAFETY: a stack of the live container
        let source_item = unsafe { source.base.item() };
        let id = self.next_id();
        let from_ref = SlotRef { container: SlotRef::INVENTORY, slot: from as u8, net_id: Self::net_id_of(source) };
        // into an empty slot the whole stack is placed; otherwise the two stacks swap
        let bytes = if empty {
            RequestEncoder::place(
                id,
                source.base.count,
                from_ref,
                SlotRef { container: SlotRef::HOTBAR, slot: to as u8, net_id: 0 },
            )
        } else {
            let to_ref = SlotRef { container: SlotRef::HOTBAR, slot: to as u8, net_id: Self::net_id_of(target) };
            RequestEncoder::swap(id, from_ref, to_ref)
        };
        // SAFETY: forwarded to the caller
        if !unsafe { self.send(now, client, player, id, &bytes) } {
            return;
        }
        inventory.swap_slots(to, from);
        self.undo_kind = Self::UNDO_SWAP;
        self.undo_slot = to;
        self.undo_from = from;
        self.undo_item = source_item;
    }

    /// Fills hotbar slot `slot` (0 to 8) with a full stack of `block`'s item from the creative menu, destroying what
    /// the slot holds. A null block, a block with no creative item, or a request that cannot be sent, is answered as
    /// refused at once.
    ///
    /// # Safety
    /// `client` and `player` must be the live client and its live local player, and `block` null or a live `Block`.
    #[allow(unsafe_code)]
    pub unsafe fn pick(&mut self, now: f64, client: *mut c_void, player: *mut c_void, block: *const c_void, slot: i32) {
        if self.await_id != 0 {
            self.finish(true);
        }
        self.answer = SwapAnswer::Refused;
        // SAFETY: the caller guarantees a live player
        let inventory = unsafe { PlayerInventory::of_player(player, &self.players) };
        let held = inventory.as_ref().and_then(|items| items.item(slot as usize));
        let (Some(inventory), Some(held), false, true) =
            (inventory.as_ref(), held, block.is_null(), (0..Self::HOTBAR_SLOTS).contains(&slot))
        else {
            curia::warn!("easy place pick not sent", { slot: slot });
            return;
        };
        // SAFETY: the caller guarantees a null or live block
        let wanted = unsafe { self.blocks.item_of(block) };
        // SAFETY: an item instance the game made; its item counter is live while it is
        let item = wanted.as_ref().map_or(std::ptr::null(), |wanted| unsafe { wanted.layout().item() });
        let aux = wanted.as_ref().map_or(0, |wanted| wanted.layout().aux_value);
        // SAFETY: the caller guarantees a live player
        let creative = unsafe { CreativeItems::of_player(player, &self.players) };
        let entry = creative.as_ref().and_then(|creative| Self::creative_entry(creative.entries(), item, aux));
        let (Some(creative), Some(entry)) = (creative.as_ref(), entry) else {
            curia::warn!("easy place pick has no creative item", { block: format!("{block:p}") });
            return;
        };
        let empty = Self::is_empty(held);
        let amount = creative.max_stack_size(&entry.item_instance);
        let net_id = entry.creative_net_id as i32;
        let id = self.next_id();
        let bytes = if empty {
            RequestEncoder::pick(
                id,
                net_id,
                amount,
                SlotRef { container: SlotRef::HOTBAR, slot: slot as u8, net_id: 0 },
            )
        } else {
            let held_ref = SlotRef { container: SlotRef::HOTBAR, slot: slot as u8, net_id: Self::net_id_of(held) };
            RequestEncoder::pick_replacing(id, net_id, amount, held_ref, held.base.count)
        };
        // SAFETY: forwarded to the caller
        if !unsafe { self.send(now, client, player, id, &bytes) } {
            return;
        }
        // SAFETY: the live held stack and the live item of the block
        unsafe {
            self.undo_stack = Some(OwnedItemStack::copy_of(held, &self.players));
            let mut predicted = OwnedItemStack::new(&self.players);
            predicted.reinit(item, i32::from(amount), i32::from(entry.item_instance.aux_value), &self.players);
            inventory.set_item(slot, &predicted);
        }
        self.undo_kind = Self::UNDO_PICK;
        self.undo_slot = slot;
        self.undo_item = item;
    }

    /// Once per client tick: a request with no answer after `REQUEST_TIMEOUT_SECONDS` is refused and its prediction
    /// undone.
    pub fn expire(&mut self, now: f64) {
        if self.await_id == 0 || now - self.sent_at < Self::REQUEST_TIMEOUT_SECONDS {
            return;
        }
        curia::info!("easy place stack request not answered", {});
        self.finish(true);
        self.answer = SwapAnswer::Refused;
    }

    /// The server's answers; the one for the request in flight sets the answer. A refusal undoes the prediction, and
    /// an accepted pick takes the net id the server gave the new stack.
    pub fn on_response(&mut self, packet: &ItemStackResponsePacketLayout) {
        if self.await_id == 0 {
            return;
        }
        let Some(response) =
            packet.responses.as_slice().iter().find(|response| response.client_request_id == self.await_id)
        else {
            return;
        };
        let accepted = response.result == 0;
        self.answer = if accepted { SwapAnswer::Accepted } else { SwapAnswer::Refused };
        if accepted {
            if self.undo_kind == Self::UNDO_PICK {
                self.adopt_pick_net_id(response);
            }
            self.finish(false);
            return;
        }
        curia::info!("easy place stack request refused", { result: response.result });
        self.finish(true);
    }

    /// The server's container open: true hides the inventory open this request asked for, and sends the close for it.
    pub fn on_container_open(&mut self, now: f64, packet: &ContainerOpenPacketLayout) -> bool {
        if now > self.open_until || packet.container_type != Self::INVENTORY_TYPE {
            return false;
        }
        self.open_until = 0.0;
        self.opened_id = i32::from(packet.container_id);
        if !self.client.is_null() {
            self.close_inventory(now, packet.container_id);
        }
        true
    }

    /// The server's container close: true hides its answer to that close.
    pub fn on_container_close(&mut self, now: f64, packet: &ContainerClosePacketLayout) -> bool {
        if now > self.close_until || i32::from(packet.container_id) != self.opened_id {
            return false;
        }
        self.close_until = 0.0;
        true
    }

    /// Drops the request in flight, its prediction undone first, and both windows; on mod disable.
    pub fn reset(&mut self) {
        self.finish(self.await_id != 0);
        self.answer = SwapAnswer::None;
        self.open_until = 0.0;
        self.close_until = 0.0;
        self.opened_id = -1;
        self.client = std::ptr::null_mut();
    }

    fn next_id(&mut self) -> i32 {
        let id = self.next_id;
        self.next_id -= Self::REQUEST_ID_STEP;
        // starts again long before the id leaves the int range
        if self.next_id > 0 || self.next_id < Self::FIRST_REQUEST_ID * 1000 {
            self.next_id = Self::FIRST_REQUEST_ID;
        }
        id
    }

    // reads the bytes into a request packet the way the network layer would, opens the inventory and sends the
    // request
    #[allow(unsafe_code)]
    unsafe fn send(
        &mut self,
        now: f64,
        client: *mut c_void,
        player: *mut c_void,
        request_id: i32,
        bytes: &[u8],
    ) -> bool {
        // SAFETY: the caller guarantees a live client
        let Some(sender) = (unsafe { PacketSender::of_client(client, &self.packets) }) else {
            curia::warn!("easy place stack request not sent, no packet sender", {});
            return false;
        };
        // SAFETY: the caller guarantees a live player
        let runtime_id = unsafe { self.runtime_id(player) };
        let packets = self.packets;
        let mut opened = false;
        let sent = sender.send_bytes(&self.packets, PacketId::ITEM_STACK_REQUEST, bytes, |sender| {
            opened = Self::open_inventory(&packets, sender, runtime_id);
        });
        match sent {
            Ok(()) => {}
            Err(PacketReadError::Refused { read_to, length }) => {
                curia::warn!("easy place stack request not read", { read_to: read_to, length: length });
                return false;
            }
            Err(error) => {
                curia::warn!("easy place stack request not created", { error: error.to_string() });
                return false;
            }
        }
        self.client = client;
        if opened {
            self.open_until = now + Self::ANSWER_WINDOW_SECONDS;
        }
        self.await_id = request_id;
        self.sent_at = now;
        self.answer = SwapAnswer::None;
        true
    }

    // the Interact OpenInventory packet for the player, as the game's own inventory key sends it
    fn open_inventory(packets: &PacketCalls, sender: &PacketSender, runtime_id: Option<u64>) -> bool {
        let Some(target) = runtime_id else { return false };
        let Some(mut packet) = GamePacket::create(packets, PacketId::INTERACT) else { return false };
        let Some(interact) = packet.payload_mut::<InteractPacketLayout>() else { return false };
        interact.action = Self::OPEN_INVENTORY;
        interact.target_id = target;
        sender.send(&packet);
        true
    }

    // the ContainerClose for the inventory the server opened, as a client close (not server-initiated)
    fn close_inventory(&mut self, now: f64, container_id: i8) {
        // SAFETY: the client a request was sent through, live while the game runs
        #[allow(unsafe_code)]
        let Some(sender) = (unsafe { PacketSender::of_client(self.client, &self.packets) }) else { return };
        let Some(mut packet) = GamePacket::create(&self.packets, PacketId::CONTAINER_CLOSE) else { return };
        let Some(close) = packet.payload_mut::<ContainerClosePacketLayout>() else { return };
        close.container_id = container_id;
        close.container_type = Self::NO_TYPE;
        close.server_initiated_close = false;
        sender.send(&packet);
        self.close_until = now + Self::ANSWER_WINDOW_SECONDS;
    }

    // ends the request in flight; with `undo`, puts back what the prediction changed when the slot still holds the
    // predicted item
    fn finish(&mut self, undo: bool) {
        let inventory = if undo { self.predicted_inventory() } else { None };
        if let Some(inventory) = inventory.as_ref() {
            if self.undo_kind == Self::UNDO_SWAP {
                inventory.swap_slots(self.undo_slot, self.undo_from);
            }
            if self.undo_kind == Self::UNDO_PICK
                && let Some(stack) = self.undo_stack.as_ref()
            {
                inventory.set_item(self.undo_slot, stack);
            }
        }
        self.await_id = 0;
        self.undo_kind = Self::UNDO_NONE;
        self.undo_slot = -1;
        self.undo_from = -1;
        self.undo_item = std::ptr::null();
        self.undo_stack = None;
    }

    // the client's inventory while the predicted slot still holds the predicted item; None otherwise
    fn predicted_inventory(&self) -> Option<PlayerInventory> {
        if self.undo_kind == Self::UNDO_NONE || self.undo_slot < 0 || self.client.is_null() {
            return None;
        }
        // SAFETY: the client a request was sent through, live while the game runs, whose vtable holds
        // getLocalPlayer at the entry's slot
        #[allow(unsafe_code)]
        let function = unsafe { RawObject::slot(self.client, self.calls.local_player_slot) }?;
        // SAFETY: the slot holds `LocalPlayer* getLocalPlayer() const`
        #[allow(unsafe_code)]
        let player = unsafe { RawObject::call_pointer(function, self.client) }?;
        // SAFETY: the client's live local player
        #[allow(unsafe_code)]
        let inventory = unsafe { PlayerInventory::of_player(player, &self.players) }?;
        let stack = inventory.item(self.undo_slot as usize)?;
        // SAFETY: a stack of the live container
        #[allow(unsafe_code)]
        let item = unsafe { stack.base.item() };
        (item == self.undo_item).then_some(inventory)
    }

    // later requests name the stack by the net id the server gave it, so the predicted stack takes that id; a slot
    // that no longer holds the picked item is left alone
    fn adopt_pick_net_id(&self, response: &ItemStackResponseInfoLayout) {
        let Some(inventory) = self.predicted_inventory() else { return };
        let slots = response
            .containers
            .as_slice()
            .iter()
            .filter(|container| container.container_name == SlotRef::HOTBAR)
            .flat_map(|container| container.slots.as_slice());
        for info in slots {
            if i32::from(info.slot) == self.undo_slot {
                inventory.set_net_id(self.undo_slot as usize, info.item_stack_net_id);
            }
        }
    }

    // the creative entry of `item` with `aux`, or the first entry of `item` with any aux
    fn creative_entry(
        entries: &[CreativeItemEntryLayout],
        item: *const c_void,
        aux: i16,
    ) -> Option<&CreativeItemEntryLayout> {
        if item.is_null() {
            return None;
        }
        let mut any_aux = None;
        for entry in entries {
            // SAFETY: an entry of the live registry; its item counter is live while the registry is
            #[allow(unsafe_code)]
            if unsafe { entry.item_instance.item() } != item {
                continue;
            }
            if entry.item_instance.aux_value == aux {
                return Some(entry);
            }
            any_aux = any_aux.or(Some(entry));
        }
        any_aux
    }

    fn is_empty(stack: &ItemStackLayout) -> bool {
        // SAFETY: a stack of the live container
        #[allow(unsafe_code)]
        let item = unsafe { stack.base.item() };
        stack.base.count == 0 || item.is_null()
    }

    fn net_id_of(stack: &ItemStackLayout) -> i32 {
        stack.net_id_variant.net_id().unwrap_or(0)
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
}
