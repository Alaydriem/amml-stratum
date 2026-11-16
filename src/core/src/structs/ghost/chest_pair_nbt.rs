// The double chest keys of one block entity NBT: `pairx`, `pairz` and whether `pairlead` is 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChestPairNbt {
    pub pair_x: i32,
    pub pair_z: i32,
    pub lead: bool,
}

impl ChestPairNbt {
    pub fn new(pair_x: i32, pair_z: i32, lead: bool) -> Self {
        Self { pair_x, pair_z, lead }
    }

    // The offset from the cell at local position `cell` to the other half of its double chest. The pair is a
    // world position when `world_origin` is set, a local one when it is None. None when it does not fit an i32.
    pub fn offset(self, cell: [i32; 3], world_origin: Option<[i32; 3]>) -> Option<[i32; 3]> {
        let origin = world_origin.unwrap_or([0; 3]);
        let x = self.pair_x.checked_sub(origin[0])?.checked_sub(cell[0])?;
        let z = self.pair_z.checked_sub(origin[2])?.checked_sub(cell[2])?;
        Some([x, 0, z])
    }
}
