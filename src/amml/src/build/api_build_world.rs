use std::cell::OnceCell;

use amml_api::{
    ApiError, BlockHandle, BlockPos, Blocks, Client, ConnectionKind, GameType, KindsInBox, LocalPlayer,
    PermissionLevel, WorldInfo,
};

use crate::services::PaletteBlocks;
use crate::traits::{BlockGrid, BuildFacts, CommandSink, PaletteGame};
use crate::types::PaletteLookup;

/// The game as the build executor sees it, through the loader's calls for one tick: the client of the tick, its local
/// player, the world epoch, and the block states. The world facts are read once, at the first call that needs them.
pub struct ApiBuildWorld<'f> {
    client: Client<'f>,
    player: Option<LocalPlayer<'f>>,
    epoch: u64,
    blocks: Blocks,
    info: OnceCell<Option<WorldInfo>>,
}

impl<'f> ApiBuildWorld<'f> {
    pub fn new(client: Client<'f>, epoch: u64, blocks: Blocks) -> Self {
        let player = client.local_player();
        Self { client, player, epoch, blocks, info: OnceCell::new() }
    }

    fn info(&self) -> Option<&WorldInfo> {
        self.info.get_or_init(|| self.client.world_info()).as_ref()
    }
}

impl BuildFacts for ApiBuildWorld<'_> {
    fn in_world(&self) -> bool {
        self.epoch != 0 && self.player.is_some() && self.client.world_active() == Some(true)
    }

    fn level(&self) -> Option<u64> {
        (self.epoch != 0).then_some(self.epoch)
    }

    fn connection_kind(&self) -> Option<ConnectionKind> {
        self.info()?.connection.as_ref().map(|connection| connection.kind)
    }

    fn in_realm(&self) -> bool {
        self.info().is_some_and(|info| info.realm == Some(true))
    }

    fn game_type(&self) -> Option<GameType> {
        self.player.as_ref()?.game_type()
    }

    fn permission_level(&self) -> Option<PermissionLevel> {
        self.player.as_ref()?.permission_level()
    }

    fn commands_enabled(&self) -> Option<bool> {
        self.info()?.commands_enabled
    }
}

impl PaletteGame for ApiBuildWorld<'_> {
    fn lookup(&self, nbt: &[u8]) -> PaletteLookup {
        PaletteBlocks::lookup(&self.blocks, nbt)
    }

    fn turn(&self, block: BlockHandle, quarters: u8) -> Option<BlockHandle> {
        self.blocks.rotated(block, quarters).ok()
    }
}

impl BlockGrid for ApiBuildWorld<'_> {
    fn kinds_in(&self, min: BlockPos, max: BlockPos, max_kinds: u32) -> Result<KindsInBox, ApiError> {
        let source = self.player.as_ref().and_then(LocalPlayer::block_source).ok_or(ApiError::Refused)?;
        source.kinds_in_box(min, max, max_kinds)
    }
}

impl CommandSink for ApiBuildWorld<'_> {
    fn send(&mut self, text: &str) -> bool {
        match self.client.send_command(text) {
            Ok(()) => true,
            Err(error) => {
                curia::warn!("build command not sent", { error: format!("{error:?}") });
                false
            }
        }
    }
}
