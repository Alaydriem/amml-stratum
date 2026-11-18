// A message of the settings panel about the last Start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildNotice {
    // The block kinds of the clear box could not be read: the player is too far from the structure.
    StandNear,
    // The clear box holds more block kinds than a build can keep track of.
    TooManyKinds,
}
