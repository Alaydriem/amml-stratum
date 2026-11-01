use amml_game::{EntryId, GameEntries};

use crate::errors::GameLayoutError;
use crate::types::StratumEntry;

/// The resolved game code of the packet path: the factory, the one-argument `Packet::read` slot, the stream the read
/// takes, and the client's packet sender. A test fills it with fake functions.
#[derive(Clone, Copy, Debug)]
pub struct PacketCalls {
    /// `std::shared_ptr<Packet> MinecraftPackets::createPacket(MinecraftPacketIds)`.
    pub create_packet: usize,
    /// Vtable slot of `Bedrock::Result<void> Packet::read(ReadOnlyBinaryStream&)`.
    pub packet_read_slot: u64,
    /// The address of the `ReadOnlyBinaryStream` vtable.
    pub stream_vtable: usize,
    /// `ReadOnlyBinaryStream::~ReadOnlyBinaryStream()`, the body.
    pub stream_dtor: usize,
    /// Vtable slot of `PacketSender& ClientInstance::getPacketSender()`.
    pub get_packet_sender_slot: u64,
    /// Vtable slot of `void PacketSender::send(Packet&)`.
    pub send_slot: u64,
}

impl PacketCalls {
    /// The calls of the build `entries` was resolved for; an error names the first entry that is missing.
    pub fn new(entries: &GameEntries) -> Result<Self, GameLayoutError> {
        let get = |id: EntryId| entries.get(id).ok_or(GameLayoutError::Missing { entry: id.as_str() });
        Ok(Self {
            create_packet: get(StratumEntry::MINECRAFT_PACKETS_CREATE_PACKET)? as usize,
            packet_read_slot: get(StratumEntry::PACKET_READ)?,
            stream_vtable: get(StratumEntry::READ_ONLY_BINARY_STREAM_VTABLE)? as usize,
            stream_dtor: get(StratumEntry::READ_ONLY_BINARY_STREAM_DTOR)? as usize,
            get_packet_sender_slot: get(StratumEntry::CLIENT_INSTANCE_GET_PACKET_SENDER)?,
            send_slot: get(StratumEntry::PACKET_SENDER_SEND)?,
        })
    }
}
