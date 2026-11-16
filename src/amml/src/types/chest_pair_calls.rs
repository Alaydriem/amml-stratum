use amml_game::GameEntries;

use crate::types::StratumEntry;

/// The resolved offsets and slot that make the lead of a chest pair a large chest: the game's chest type check, the
/// actor's position and the chest pair fields. The optional feature `stratum.large_chests`; without it a lead chest
/// draws as a single chest. A test fills it with fake values.
#[derive(Clone, Copy, Debug)]
pub struct ChestPairCalls {
    /// Vtable slot of `bool BlockActor::isType(BlockActorType) const`.
    pub is_type_slot: u64,
    /// Offset of `mPosition` (a BlockPos) in `BlockActor`.
    pub actor_position: u64,
    /// Offset of the pair flags byte in `ChestBlockActor`, the byte whose bit 1 gives the pair axis.
    pub chest_pair_flags: u64,
    /// Offset of `mLargeChestPaired` (a `ChestBlockActor*`) in `ChestBlockActor`.
    pub chest_large_paired: u64,
    /// Offset of `mLargeChestPairedPosition` (a BlockPos) in `ChestBlockActor`.
    pub chest_large_paired_position: u64,
}

impl ChestPairCalls {
    /// The chest pair calls of the build `entries` was resolved for; `None` when any entry of the feature is missing.
    pub fn new(entries: &GameEntries) -> Option<Self> {
        Some(Self {
            is_type_slot: entries.get(StratumEntry::BLOCK_ACTOR_IS_TYPE)?,
            actor_position: entries.get(StratumEntry::BLOCK_ACTOR_POSITION)?,
            chest_pair_flags: entries.get(StratumEntry::CHEST_BLOCK_ACTOR_PAIR_FLAGS)?,
            chest_large_paired: entries.get(StratumEntry::CHEST_BLOCK_ACTOR_LARGE_CHEST_PAIRED)?,
            chest_large_paired_position: entries.get(StratumEntry::CHEST_BLOCK_ACTOR_LARGE_CHEST_PAIRED_POSITION)?,
        })
    }
}
