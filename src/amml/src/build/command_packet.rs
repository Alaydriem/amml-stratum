use std::ffi::c_void;

use amml_game::GameHeap;
use amml_msvc::MsvcString;

use crate::errors::{CommandSendError, GameLayoutError};
use crate::game::{GamePacket, PacketId, PacketSender};
use crate::types::{PacketCalls, StratumEntry};

/// Sends a command as the game's chat does: a `CommandRequestPacket` from the game's factory, with the text written
/// as a game heap string over the empty `mCommand` the factory left, sent through the client's packet sender. The
/// packet owns the text from then on and frees it with its last reference. Game thread.
#[derive(Clone, Copy, Debug)]
pub struct CommandPacket {
    packets: PacketCalls,
    command_at: usize,
}

impl CommandPacket {
    /// A sender for the packet whose `mCommand` is at `command_at` in a packet of `packet_size` bytes; an error when
    /// the string would end past the packet.
    pub fn new(packets: PacketCalls, command_at: u64, packet_size: u64) -> Result<Self, GameLayoutError> {
        let string_size = std::mem::size_of::<MsvcString>() as u64;
        let fits = command_at.checked_add(string_size).is_some_and(|end| end <= packet_size);
        let at = usize::try_from(command_at).ok().filter(|_| fits);
        match at {
            Some(command_at) => Ok(Self { packets, command_at }),
            None => Err(GameLayoutError::Misfit {
                entry: StratumEntry::COMMAND_REQUEST_PACKET_M_COMMAND.as_str(),
                expected: packet_size.saturating_sub(string_size),
                found: command_at,
            }),
        }
    }

    /// Sends `text` as a command of the local player. Everything the send needs is looked up before the packet is
    /// made, so a failed step leaves nothing to undo but the text, which is given back to the heap.
    ///
    /// # Safety
    /// `client` must be null or the live `ClientInstance` of the build the calls were resolved for.
    #[allow(unsafe_code)]
    pub unsafe fn send(&self, client: *mut c_void, text: &str) -> Result<(), CommandSendError> {
        let heap = GameHeap::msvc().ok_or(CommandSendError::NoHeap)?;
        // SAFETY: forwarded to the caller
        let sender = unsafe { PacketSender::of_client(client, &self.packets) }.ok_or(CommandSendError::NoSender)?;
        // SAFETY: the heap is the game's operator new and delete pair
        let mut string = unsafe { MsvcString::new_in(&heap, text) }.map_err(|_| CommandSendError::NoText)?;
        let Some(packet) = GamePacket::create(&self.packets, PacketId::COMMAND_REQUEST) else {
            // SAFETY: the string came from `new_in` with this heap and is not used again
            unsafe { string.release(&heap) };
            return Err(CommandSendError::NotMade);
        };
        // SAFETY: `packet` is the live CommandRequestPacket the factory made, `command_at` is the offset of its
        // `mCommand`, which holds an empty small string with no block to free, and `new` checked that the string lies
        // inside the packet. The string moves into the packet, which frees its block in its destructor, so it is not
        // released here
        unsafe { packet.as_ptr().cast::<u8>().add(self.command_at).cast::<MsvcString>().write_unaligned(string) };
        if sender.send(&packet) { Ok(()) } else { Err(CommandSendError::NotSent) }
    }
}
