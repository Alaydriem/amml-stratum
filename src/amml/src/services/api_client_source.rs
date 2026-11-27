use std::cell::OnceCell;

use amml_api::{Client, ConnectionKind, LocalPlayer, Options, WorldInfo};

use crate::traits::{ClientSource, WorldSource};
use crate::types::{ConnectionInfo, PlayerSpot};

// What the frame status and the world key read for one tick, through the loader's calls on the client of the tick.
// The world facts are read once, at the first call that needs them, so a tick that does not read the world key makes no
// world call.
pub struct ApiClientSource<'f> {
    client: Client<'f>,
    player: Option<LocalPlayer<'f>>,
    epoch: u64,
    options: Options,
    // the address and port the player typed for the server join that started this world
    typed: Option<(String, u16)>,
    info: OnceCell<Option<WorldInfo>>,
}

impl<'f> ApiClientSource<'f> {
    pub fn new(client: Client<'f>, epoch: u64, options: Options, typed: Option<(String, u16)>) -> Self {
        let player = client.local_player();
        Self { client, player, epoch, options, typed, info: OnceCell::new() }
    }

    fn info(&self) -> Option<&WorldInfo> {
        self.info.get_or_init(|| self.client.world_info()).as_ref()
    }
}

impl WorldSource for ApiClientSource<'_> {
    fn level(&self) -> Option<u64> {
        (self.epoch != 0).then_some(self.epoch)
    }

    fn in_realm(&self) -> bool {
        self.info().is_some_and(|info| info.realm == Some(true))
    }

    // the loader's connection, with the typed address of a remote world when the server join gave one for its port
    fn connection(&self) -> Option<ConnectionInfo> {
        let mut connection = ConnectionInfo::from(self.info()?.connection.clone()?);
        if let Some((url, port)) = &self.typed
            && connection.kind == ConnectionKind::Remote
            && (*port == connection.port || connection.port == 0)
        {
            connection.url.clone_from(url);
            connection.port = *port;
        }
        Some(connection)
    }

    fn local_folder(&self) -> Option<String> {
        let folder = self.info()?.storage_folder.as_ref()?;
        folder.file_name().and_then(|name| name.to_str()).map(str::to_owned)
    }

    fn level_name(&self) -> Option<String> {
        self.info()?.level_name.clone()
    }

    fn seed(&self) -> Option<u64> {
        self.info()?.seed
    }
}

impl ClientSource for ApiClientSource<'_> {
    fn is_world_active(&self) -> bool {
        self.client.world_active() == Some(true)
    }

    fn player(&self) -> Option<PlayerSpot> {
        let player = self.player.as_ref()?;
        let position = player.position()?;
        let dimension = player.dimension().and_then(|dimension| dimension.id()).unwrap_or(0);
        Some(PlayerSpot { dimension, feet: PlayerSpot::feet_of([position.x, position.y, position.z]) })
    }

    fn menu_open(&self) -> bool {
        self.client.showing_menu() == Some(true)
    }

    fn view_distance_chunks(&self) -> i32 {
        self.options.view_distance_chunks(&self.client).map_or(0, |chunks| i32::try_from(chunks).unwrap_or(i32::MAX))
    }
}
