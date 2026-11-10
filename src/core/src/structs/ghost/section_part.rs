// Which part of a 16 x 16 x 16 section a mesh job covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SectionPart {
    Whole,
    // The 16 x 8 x 16 half with local y 0 to 7.
    Lower,
    // The 16 x 8 x 16 half with local y 8 to 15.
    Upper,
    // One merged mesh of the mini ghost: the cells of a group of sections.
    MiniatureGroup,
    // The single structure block that marks the ghost's lowest corner.
    Marker,
}

impl SectionPart {
    // Local y of the part's lowest cell.
    pub fn y_offset(self) -> i32 {
        match self {
            Self::Whole | Self::Lower | Self::MiniatureGroup | Self::Marker => 0,
            Self::Upper => 8,
        }
    }

    // Height of the part in cells.
    pub fn height(self) -> u32 {
        match self {
            Self::Whole | Self::MiniatureGroup => 16,
            Self::Lower | Self::Upper => 8,
            Self::Marker => 1,
        }
    }

    // True for the part of the mini ghost.
    pub fn is_miniature(self) -> bool {
        self == Self::MiniatureGroup
    }
}
