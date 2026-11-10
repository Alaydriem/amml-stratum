// Quarter turns of a placed structure around the vertical axis, clockwise seen from above, as
// Minecraft's Rotate90 turns: at R90 the +X (east) side faces +Z (south), so a cell offset
// (x, y, z) from the turn's centre cell becomes (-z, y, x). The centre cell stays where it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Rotation {
    R0,
    R90,
    R180,
    R270,
}

impl Rotation {
    // The clockwise turn in degrees.
    pub fn degrees(self) -> u16 {
        match self {
            Self::R0 => 0,
            Self::R90 => 90,
            Self::R180 => 180,
            Self::R270 => 270,
        }
    }

    // The clockwise quarter turns, 0 to 3.
    pub fn quarters(self) -> u8 {
        match self {
            Self::R0 => 0,
            Self::R90 => 1,
            Self::R180 => 2,
            Self::R270 => 3,
        }
    }

    // Turns by `quarters` quarter turns, clockwise when positive. Wraps both ways.
    pub fn turned(self, quarters: i8) -> Rotation {
        match (i32::from(self.quarters()) + i32::from(quarters)).rem_euclid(4) {
            0 => Self::R0,
            1 => Self::R90,
            2 => Self::R180,
            _ => Self::R270,
        }
    }

    // A cell offset from the turn's centre cell, turned.
    pub fn turn_offset(self, offset: [i64; 3]) -> [i64; 3] {
        let [x, y, z] = offset;
        match self {
            Self::R0 => [x, y, z],
            Self::R90 => [-z, y, x],
            Self::R180 => [-x, y, -z],
            Self::R270 => [z, y, -x],
        }
    }

    // The lowest corner, from the turn's centre cell, of a box of `size` cells whose cell (0, 0, 0)
    // sits at `offset` from the centre cell before the turn, and the size of that box after the turn.
    pub fn turn_box(self, offset: [i64; 3], size: [u32; 3]) -> ([i64; 3], [u32; 3]) {
        let turned = self.turn_offset(offset);
        let corner = self.turn_corner(size);
        ([0, 1, 2].map(|axis| turned[axis] + corner[axis]), self.turn_size(size))
    }

    // The lowest corner of a box of `size` cells from (0, 0, 0) after the turn, from the box's own
    // cell (0, 0, 0).
    pub fn turn_corner(self, size: [u32; 3]) -> [i64; 3] {
        let far = size.map(|side| i64::from(side) - 1);
        self.turn_offset(far).map(|value| value.min(0))
    }

    // The size of a box after the turn: a quarter turn swaps X and Z.
    pub fn turn_size(self, size: [u32; 3]) -> [u32; 3] {
        match self {
            Self::R0 | Self::R180 => size,
            Self::R90 | Self::R270 => [size[2], size[1], size[0]],
        }
    }
}
