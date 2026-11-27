use amml_api::{ApiError, BlockPos, KindsInBox};

// What the kinds scan reads of the world's blocks: the distinct block types of one box, as the game names them
pub trait BlockGrid {
    // the kinds of the box from `min` to `max`, both included; the box holds at most `BlockSource::KINDS_CELL_CAP`
    // cells
    fn kinds_in(&self, min: BlockPos, max: BlockPos, max_kinds: u32) -> Result<KindsInBox, ApiError>;
}
