// What surrounds a spot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpotWorld {
    // Indexed in `Face::ALL` order. `against[i]` is true when the neighbour at
    // `spot + Face::ALL[i].normal()` can be placed against.
    pub against: [bool; 6],
}
