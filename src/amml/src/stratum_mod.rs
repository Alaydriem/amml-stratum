use std::sync::{Mutex, PoisonError};

use amml_api::amml_abi::{
    AppFocusLost, BlockChanged, BlockEntityPass, BuildBlock, ClientTick, DimensionChanging, InjectedInputReleased,
    InputLeaseLost, InputLeaseResumed, InputLeaseSuspended, InventoryRequestDone, ModsLoaded, MouseInput, ProviderGone,
    ProviderReady, ScreenshotTaken, ScreenshotWritten, ServerJoinStarting, UsePressed, WindowFocus, WorldJoined,
    WorldLeft, WorldRendered,
};
use amml_api::{AmmlMod, ApiError, Area, Ctx, DisableCtx, Uses};

use crate::services::{StratumListeners, StratumRuntime};
use crate::types::StratumRunning;

// what the enabled mod runs; `disable` takes it out of the lock, then drops the listeners and stops the runtime
static RUNNING: Mutex<Option<StratumRunning>> = Mutex::new(None);

// the AMML entry point: enable starts the runtime with its manager link and adds the listeners; disable removes the
// listeners, closes the picker and stops the core. The loader frees the meshes, groups, block entity ghosts and block
// handles of a disabled mod itself
pub struct StratumMod;

impl StratumMod {
    // whether this build runs the in-game gate
    pub const GATE: bool = cfg!(feature = "gate");

    // the features of `USES`, each logged as on at enable: the loader enables the mod only when every row is served
    pub const FEATURES: [&'static str; 7] =
        ["core", "ghost", "block_changes", "block_entities", "easy_place", "picker", "build"];
}

impl AmmlMod for StratumMod {
    const NAMESPACE: &'static str = "alaydriem";
    const NAME: &'static str = "stratum";
    const USES: Uses = amml_api::amml_uses! {
        feature "core" {
            required ClientTick;
            required WorldJoined;
            required WorldLeft;
            required DimensionChanging;
            required ServerJoinStarting;
            required ProviderReady;
            required ProviderGone;
            required ModsLoaded;
            required InputLeaseSuspended;
            required InputLeaseResumed;
            required InputLeaseLost;
            required InjectedInputReleased;
            required ScreenshotTaken;
            required ScreenshotWritten;
            required area Area::INPUT_LEASE;
            required area Area::CLIENT_READ;
            required area Area::CLIENT_WORLD_INFO;
            required area Area::ACTOR_READ.at(4);
            required area Area::OPTIONS_VIEW_DISTANCE;
        }
        feature "ghost" {
            required WorldRendered;
            required area Area::RENDER_WORLD_CANVAS.at(3);
            required area Area::RENDER_BLOCK_MESHER;
            required area Area::WORLD_BLOCK_STATES;
            required area Area::RENDER_WATER_TINT;
        }
        feature "block_changes" {
            required BlockChanged;
        }
        feature "block_entities" {
            required BlockEntityPass;
            required area Area::RENDER_BLOCK_ENTITIES;
            required area Area::RENDER_CHEST_PAIR;
            required area Area::RENDER_SKULL_POSE;
        }
        feature "easy_place" {
            required UsePressed;
            required BuildBlock;
            required InventoryRequestDone;
            required MouseInput;
            required WindowFocus;
            required area Area::WORLD_BLOCKS;
            required area Area::PLAYER_BUILD;
            required area Area::PLAYER_PICK_RANGE;
            required area Area::PLAYER_HIT_RESULT;
            required area Area::HUD_SELECT_HOTBAR;
            required area Area::INVENTORY_READ.at(3);
            required area Area::INVENTORY_REQUEST.at(2);
        }
        feature "picker" {
            required AppFocusLost;
        }
        feature "build" {
            required area Area::CLIENT_SEND_COMMAND;
            required area Area::ACTOR_GAME_TYPE;
            required area Area::ACTOR_PERMISSION;
            required area Area::WORLD_KINDS_IN_BOX;
        }
    };

    // no game call here: the standalone host enables before the game runs
    fn enable(ctx: &Ctx) -> Result<(), ApiError> {
        let log = ctx.log();
        let runtime = StratumRuntime::start(ctx)?;
        let listeners = match StratumListeners::start(ctx, &runtime) {
            Ok(listeners) => listeners,
            Err(error) => {
                runtime.stop();
                log.error("stratum off: a listener was refused", &[("error", &format!("{error:?}"))]);
                return Err(error);
            }
        };
        *RUNNING.lock().unwrap_or_else(PoisonError::into_inner) = Some(StratumRunning { runtime, listeners });
        for feature in Self::FEATURES {
            log.info("feature on", &[("name", feature)]);
        }
        if Self::GATE {
            log.info("gate on: the first world builds one stone mesh and two block entity ghosts", &[]);
        }
        Ok(())
    }

    // the listeners go first, outside the lock, so no event runs while the core stops
    fn disable(_ctx: &Ctx, _how: &DisableCtx) {
        let running = RUNNING.lock().unwrap_or_else(PoisonError::into_inner).take();
        if let Some(StratumRunning { runtime, listeners }) = running {
            listeners.stop();
            runtime.stop();
        }
    }
}
