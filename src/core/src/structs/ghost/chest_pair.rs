// The direction from the lead half of a double chest to its other half, in world axes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChestPair {
    PlusX,
    MinusX,
    PlusZ,
    MinusZ,
}

impl ChestPair {
    // The direction of a one block offset on X or Z at the same height, or None for any other offset.
    pub fn from_offset(offset: [i32; 3]) -> Option<Self> {
        match offset {
            [1, 0, 0] => Some(Self::PlusX),
            [-1, 0, 0] => Some(Self::MinusX),
            [0, 0, 1] => Some(Self::PlusZ),
            [0, 0, -1] => Some(Self::MinusZ),
            _ => None,
        }
    }

    // The offset from the lead half to the other half.
    pub fn offset(self) -> [i32; 3] {
        match self {
            Self::PlusX => [1, 0, 0],
            Self::MinusX => [-1, 0, 0],
            Self::PlusZ => [0, 0, 1],
            Self::MinusZ => [0, 0, -1],
        }
    }
}
