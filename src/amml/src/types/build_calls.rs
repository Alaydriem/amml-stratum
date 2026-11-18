use amml_game::{EntryId, GameEntries};

use crate::build::CommandPacket;
use crate::errors::GameLayoutError;
use crate::types::{GhostCalls, PacketCalls, PlayerCalls, StratumEntry, UseCalls};

/// The resolved game code and offsets of the build: the command packet, the three condition reads, the block reads of
/// the clear and the palette lookups. A test fills it from fake entries.
#[derive(Clone, Copy, Debug)]
pub struct BuildCalls {
    pub command: CommandPacket,
    pub players: PlayerCalls,
    pub uses: UseCalls,
    pub ghost: GhostCalls,
    /// `PlayerPermissionLevel Player::getPlayerPermissionLevel() const`: rcx the player, the level in al.
    pub permission_level: usize,
    /// Offset of `LevelData::mCommandsEnabled`, a byte.
    pub commands_enabled: u64,
    /// Vtable slot of `LevelChunk* BlockSource::getChunk(int x, int z) const`.
    pub get_chunk_slot: u64,
    /// Offset of `mNameInfo.mFullName` (a `HashedString`) in `BlockType`.
    pub block_type_full_name: u64,
}

impl BuildCalls {
    /// The calls of the build `entries` was resolved for; an error names the first entry that is missing, or the
    /// command string that does not lie inside the command packet.
    pub fn new(entries: &GameEntries, ghost: &GhostCalls) -> Result<Self, GameLayoutError> {
        let get = |id: EntryId| entries.get(id).ok_or(GameLayoutError::Missing { entry: id.as_str() });
        let packets = PacketCalls::new(entries)?;
        let command = CommandPacket::new(
            packets,
            get(StratumEntry::COMMAND_REQUEST_PACKET_M_COMMAND)?,
            get(StratumEntry::COMMAND_REQUEST_PACKET_SIZE)?,
        )?;
        Ok(Self {
            command,
            players: PlayerCalls::new(entries)?,
            uses: UseCalls::new(entries)?,
            ghost: *ghost,
            permission_level: get(StratumEntry::PLAYER_GET_PLAYER_PERMISSION_LEVEL)? as usize,
            commands_enabled: get(StratumEntry::LEVEL_DATA_COMMANDS_ENABLED)?,
            get_chunk_slot: get(StratumEntry::BLOCK_SOURCE_GET_CHUNK)?,
            block_type_full_name: get(StratumEntry::BLOCK_TYPE_FULL_NAME)?,
        })
    }
}
