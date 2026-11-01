// One face of a block. North is -Z, West is -X, Down is -Y, as in the game. The variants are in the game's
// facing order, and `index` is the position in that order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Face {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl Face {
    // Every face, in the order of `index`.
    pub const ALL: [Face; 6] = [Face::Down, Face::Up, Face::North, Face::South, Face::West, Face::East];

    // The position of this face in `ALL`.
    pub fn index(self) -> usize {
        self as usize
    }

    // The step from a block to its neighbour on this face.
    pub fn normal(self) -> [i32; 3] {
        match self {
            Self::Down => [0, -1, 0],
            Self::Up => [0, 1, 0],
            Self::North => [0, 0, -1],
            Self::South => [0, 0, 1],
            Self::West => [-1, 0, 0],
            Self::East => [1, 0, 0],
        }
    }

    pub fn opposite(self) -> Face {
        match self {
            Self::Down => Self::Up,
            Self::Up => Self::Down,
            Self::North => Self::South,
            Self::South => Self::North,
            Self::West => Self::East,
            Self::East => Self::West,
        }
    }

    // The key the game's `blocks.json` uses for this face.
    pub fn json_key(self) -> &'static str {
        match self {
            Self::Down => "down",
            Self::Up => "up",
            Self::North => "north",
            Self::South => "south",
            Self::West => "west",
            Self::East => "east",
        }
    }
}
