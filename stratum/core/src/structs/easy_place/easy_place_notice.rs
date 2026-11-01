// A message easy place asks the player to see.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EasyPlaceNotice {
    Missing { palette_id: u16 },
    NoNeighbour,
}
