/// The render layers one block is tessellated in. The game tessellates a block once for each layer queue it is in:
/// its main layer (`Block::getRenderLayer`), then each bit of `BlockType::getExtraRenderLayers()`. Shapes such as
/// honey, slime and the beacon draw only on some layers; shapes such as a full cube ignore the layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct LayerPasses {
    main: u8,
    extra: u32,
}

impl LayerPasses {
    /// `BlockRenderLayer` values below this are real layers; the chunk builder has one queue for each.
    pub const LAYER_COUNT: u32 = 22;

    pub fn new(main: u8, extra: u32) -> Self {
        Self { main, extra }
    }

    /// The main layer first, then each extra layer that is not the main one, in rising order.
    pub fn layers(&self) -> impl Iterator<Item = i32> {
        let main = self.main;
        let extra = self.extra_bits();
        let layers = 0..Self::LAYER_COUNT as i32;
        std::iter::once(i32::from(main)).chain(layers.filter(move |bit| extra & (1 << bit) != 0))
    }

    /// The number of passes `layers` gives.
    pub fn count(&self) -> usize {
        1 + self.extra_bits().count_ones() as usize
    }

    // the extra bits that name a real layer other than the main one
    fn extra_bits(&self) -> u32 {
        let real = (1u32 << Self::LAYER_COUNT) - 1;
        let main = 1u32.checked_shl(u32::from(self.main)).unwrap_or(0);
        self.extra & real & !main
    }
}
