use crate::easy_place::SlotRef;

/// Writes the bytes of one item stack request the way the game's `BinaryStream` writers do, so that
/// `Packet::read` of an `ItemStackRequestPacket` reads them: one request, its id, its actions, then no strings to
/// filter and their origin -1. Each action starts with its index in the game's action variant, then its type.
pub struct RequestEncoder;

impl RequestEncoder {
    const ACTION_TAKE: u8 = 0;
    const ACTION_PLACE: u8 = 1;
    const ACTION_SWAP: u8 = 2;
    const ACTION_DESTROY: u8 = 4;
    // CraftCreative is type 14 at variant index 12: the deprecated actions 7 and 8 are not in the variant
    const ACTION_CRAFT_CREATIVE: u8 = 14;
    const ACTION_CRAFT_CREATIVE_INDEX: u8 = 12;
    const CRAFTS: u8 = 1;
    const ORIGIN_NONE: i32 = -1;

    /// Exchanges the stacks of `from` and `to`; both slots hold a stack.
    pub fn swap(request_id: i32, from: SlotRef, to: SlotRef) -> Vec<u8> {
        let mut out = Self::start(request_id, 1);
        Self::write_action(&mut out, Self::ACTION_SWAP, Self::ACTION_SWAP);
        Self::write_slot(&mut out, from);
        Self::write_slot(&mut out, to);
        Self::write_end(&mut out);
        out
    }

    /// Moves the whole stack of `from`, `count` items, into the empty slot `to`.
    pub fn place(request_id: i32, count: u8, from: SlotRef, to: SlotRef) -> Vec<u8> {
        let mut out = Self::start(request_id, 1);
        Self::write_action(&mut out, Self::ACTION_PLACE, Self::ACTION_PLACE);
        out.push(count);
        Self::write_slot(&mut out, from);
        Self::write_slot(&mut out, SlotRef { net_id: 0, ..to });
        Self::write_end(&mut out);
        out
    }

    /// Fills the empty slot `to` with `count` items of the creative item whose creative net id is `item_net_id`.
    pub fn pick(request_id: i32, item_net_id: i32, count: u8, to: SlotRef) -> Vec<u8> {
        let mut out = Self::start(request_id, 2);
        Self::write_create(&mut out, request_id, item_net_id, count, to);
        out
    }

    /// As `pick`, into `held`, which holds `held_count` items that are destroyed first.
    pub fn pick_replacing(request_id: i32, item_net_id: i32, count: u8, held: SlotRef, held_count: u8) -> Vec<u8> {
        let mut out = Self::start(request_id, 3);
        Self::write_action(&mut out, Self::ACTION_DESTROY, Self::ACTION_DESTROY);
        out.push(held_count);
        Self::write_slot(&mut out, held);
        Self::write_create(&mut out, request_id, item_net_id, count, held);
        out
    }

    /// `BinaryStream::writeVarInt`: the zigzag form of `value`, as an unsigned varint.
    pub fn write_var_int(out: &mut Vec<u8>, value: i32) {
        Self::write_unsigned_var_int(out, ((value << 1) ^ (value >> 31)) as u32);
    }

    /// `BinaryStream::writeUnsignedVarInt`: 7 bits per byte, low bits first, the high bit set on every byte but
    /// the last.
    pub fn write_unsigned_var_int(out: &mut Vec<u8>, mut value: u32) {
        while value >= 0x80 {
            out.push((value as u8 & 0x7F) | 0x80);
            value >>= 7;
        }
        out.push(value as u8);
    }

    // one request, its id and the count of its actions
    fn start(request_id: i32, actions: u32) -> Vec<u8> {
        let mut out = Vec::with_capacity(48);
        Self::write_unsigned_var_int(&mut out, 1);
        Self::write_var_int(&mut out, request_id);
        Self::write_unsigned_var_int(&mut out, actions);
        out
    }

    // the creative craft, then the take of the crafted stack into the empty `to`; the crafted stack's net id is the
    // request's own id
    fn write_create(out: &mut Vec<u8>, request_id: i32, item_net_id: i32, count: u8, to: SlotRef) {
        Self::write_action(out, Self::ACTION_CRAFT_CREATIVE_INDEX, Self::ACTION_CRAFT_CREATIVE);
        Self::write_unsigned_var_int(out, item_net_id as u32);
        out.push(Self::CRAFTS);
        Self::write_action(out, Self::ACTION_TAKE, Self::ACTION_TAKE);
        out.push(count);
        let created =
            SlotRef { container: SlotRef::CREATED_OUTPUT, slot: SlotRef::CREATED_OUTPUT_SLOT, net_id: request_id };
        Self::write_slot(out, created);
        Self::write_slot(out, SlotRef { net_id: 0, ..to });
        Self::write_end(out);
    }

    fn write_action(out: &mut Vec<u8>, index: u8, kind: u8) {
        out.push(index);
        out.push(kind);
    }

    // container, no dynamic id, slot, net id as a little-endian signed int
    fn write_slot(out: &mut Vec<u8>, slot: SlotRef) {
        out.push(slot.container);
        out.push(0);
        out.push(slot.slot);
        out.extend_from_slice(&slot.net_id.to_le_bytes());
    }

    // no strings to filter, and their origin
    fn write_end(out: &mut Vec<u8>) {
        Self::write_unsigned_var_int(out, 0);
        out.extend_from_slice(&Self::ORIGIN_NONE.to_le_bytes());
    }
}
