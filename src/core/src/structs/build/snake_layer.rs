// The shape of one layer, with its cells numbered in snake order.
#[derive(Clone, Copy)]
pub(crate) struct SnakeLayer {
    pub(crate) long_is_x: bool,
    pub(crate) long_len: u32,
    pub(crate) short_len: u32,
}

impl SnakeLayer {
    // The x and z of a snake index.
    pub(crate) fn cell_of(self, index: u32) -> [u32; 2] {
        let row = index / self.long_len;
        let along = self.along(index);
        if self.long_is_x { [along, row] } else { [row, along] }
    }

    pub(crate) fn along(self, index: u32) -> u32 {
        let row = index / self.long_len;
        let offset = index % self.long_len;
        if row.is_multiple_of(2) { offset } else { self.long_len - 1 - offset }
    }

    pub(crate) fn index_at(self, row: u32, along: u32) -> u32 {
        row * self.long_len + if row.is_multiple_of(2) { along } else { self.long_len - 1 - along }
    }
}
