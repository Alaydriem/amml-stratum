/// A request packet was not made or did not read the bytes the mod wrote, so it is not sent.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum PacketReadError {
    #[error("the game made no packet of that type")]
    NotMade,
    #[error("the packet has no read function")]
    NoReadSlot,
    #[error("the packet refused the bytes: read {read_to} of {length}")]
    Refused { read_to: u64, length: usize },
}
