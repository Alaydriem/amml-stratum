use crate::traits::{BlockGrid, BuildFacts, CommandSink, PaletteGame};

// The game as the build executor sees it: the conditions, the palette, the blocks and the command packet
pub trait BuildWorld: BuildFacts + PaletteGame + BlockGrid + CommandSink {}

impl<T: BuildFacts + PaletteGame + BlockGrid + CommandSink> BuildWorld for T {}
