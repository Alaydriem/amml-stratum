use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

use amm_client::link::LinkState;
use amml_api::amml_abi::{AmmlOrigin, BlockChanged, BlockEntityPass, BuildBlock, MouseInput, WorldRendered};
use amml_api::{
    ApiError, Area, BlockChangedExt, BuildBlockExt, Ctx, Event, EventCtx, Logger, MouseAction, MouseButton,
    MouseInputExt, RequestId, RequestOutcome,
};
use stratum_core::structs::build::BuildConditions;
use stratum_core::structs::easy_place::UseCommand;

use crate::build::{ApiBuildWorld, BuildExecutor};
use crate::easy_place::{UseCycle, UseDriver, UseFlags};
#[cfg(feature = "gate")]
use crate::services::GateRunner;
use crate::services::{ApiClientSource, BlockChangeQueue, ManagerSession, SharedGhost, StratumHandle};
use crate::traits::{BuildCore, BuildFacts, UseCore};
#[cfg(feature = "gate")]
use crate::types::PlayerSpot;
use crate::types::{CoreState, PathList, QueuedChange};

// What the listeners share for one enable: the core with its link, frame reader, picker and build; the ghost renderer;
// easy place; and, with the gate, the in-game proof. The tick, the world render, the block entity pass, the block
// changes and the use press run on the game thread; the focus loss and the input events read only atomics; the build
// event also runs on the local world's server thread and reads only easy place's flags. A listener releases every lock
// before a loader action (`build_at`, `select_hotbar_slot`, a request, `send_command`), which raises events on the same
// stack, and the block change listener only tries the core's lock, so a change raised inside an action waits in the
// queue for the next tick.
pub struct StratumRuntime {
    ctx: Ctx,
    log: Logger,
    data_dir: PathBuf,
    core: Mutex<CoreState>,
    // the ghost frames and reports of the running core, for the world render
    ghost: Mutex<Option<Arc<SharedGhost>>>,
    renderer: Mutex<crate::ghost::GhostRenderer>,
    // the block changes that came while the core was held, for the next tick
    block_changes: BlockChangeQueue,
    driver: Mutex<UseDriver>,
    flags: UseFlags,
    // the core's answer at the end of the last tick: the library keeps itself open through the file picker
    pause_unwanted: AtomicBool,
    // the address and port the player typed for the last server join, until the world is left
    typed_server: Mutex<Option<(String, u16)>>,
    // the start of the steady clock the frame status carries
    start: Instant,
    #[cfg(feature = "gate")]
    gate: Mutex<GateRunner>,
}

impl StratumRuntime {
    // the areas the build needs, besides the ones every feature shares
    const BUILD_AREAS: [Area; 4] =
        [Area::CLIENT_SEND_COMMAND, Area::ACTOR_GAME_TYPE, Area::ACTOR_PERMISSION, Area::WORLD_KINDS_IN_BOX];

    // enable: starts the manager link with the library and HUD frame handler; the core starts on the first tick after
    // the link connects. No game call is made here
    pub fn start(ctx: &Ctx) -> Result<Arc<Self>, ApiError> {
        let log = ctx.log();
        let Some(data_dir) = ctx.dirs().data_dir().map(|dir| Self::absolute(&dir)) else {
            log.error("stratum off: the loader gives no data folder", &[]);
            return Err(ApiError::Refused);
        };
        let shared = StratumHandle::shared_for(&data_dir);
        let link = ManagerSession::start_link(ctx, &shared).map_err(|error| {
            log.error("stratum off: the manager link did not start", &[("error", &error.to_string())]);
            ApiError::Refused
        })?;
        let build = BuildExecutor::new(Self::BUILD_AREAS.iter().all(|area| ctx.supports(*area)));
        Ok(Arc::new(Self {
            ctx: *ctx,
            log,
            data_dir,
            core: Mutex::new(CoreState::new(link, shared, build)),
            ghost: Mutex::new(None),
            renderer: Mutex::new(crate::ghost::GhostRenderer::new(ctx.supports(Area::RENDER_WATER_TINT))),
            block_changes: BlockChangeQueue::new(),
            driver: Mutex::new(UseDriver::new()),
            flags: UseFlags::new(),
            pause_unwanted: AtomicBool::new(false),
            typed_server: Mutex::new(None),
            start: Instant::now(),
            #[cfg(feature = "gate")]
            gate: Mutex::new(GateRunner::new(log, ctx.dirs().instance_dir().as_deref())),
        }))
    }

    fn now(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    // `path` made absolute against the working folder; the manager and the library need absolute paths
    fn absolute(path: &Path) -> PathBuf {
        std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
    }

    // `ClientTick`: the core's tick with the frame status, then the build's requests and the use call; the commands
    // and the use command are carried out after the lock is released, and what they gave goes back under it
    pub fn tick(&self, at: &EventCtx) {
        let now = self.now();
        let epoch = at.epoch.0;
        let Some(client) = at.client() else { return };
        let player = client.local_player();
        let (commands, command, counts) = {
            let Ok(mut core) = self.core.lock() else { return };
            self.connect(&mut core);
            let CoreState { handle, frame, picker, last_tick_ok, build, conditions, .. } = &mut *core;
            let Some(handle) = handle.as_mut() else { return };
            self.block_changes.drain(|change| Self::apply_change(handle, change));
            let dropped = self.block_changes.take_dropped();
            if dropped > 0 {
                curia::warn!("block changes dropped, the block change queue was full", { dropped: dropped });
            }
            let Some(source_client) = at.client() else { return };
            let typed = self.typed_server.lock().ok().and_then(|typed| typed.clone());
            let source = ApiClientSource::new(source_client, epoch, self.ctx.options(), typed);
            let status = frame.read(&source, now);
            let menu_open = status.menu_open;
            let ticked = handle.with_state(|state| state.tick(status)).is_some();
            if let Some((kind, key, in_realm, reads)) = frame.world_key_mut().take_chosen() {
                let (in_realm, reads) = (in_realm.to_string(), reads.to_string());
                self.log.info("world key", &[("kind", kind), ("key", key), ("in_realm", &in_realm), ("reads", &reads)]);
            }
            if let Some(answer) = picker.deliver() {
                let files = PathList::new(answer.paths).into_paths();
                if handle.with_state(|state| state.files_picked(files)).is_none() {
                    curia::warn!("picked files not handed to the library", {});
                }
            }
            if ticked && let Some(world_client) = at.client() {
                let world = ApiBuildWorld::new(world_client, epoch, self.ctx.blocks());
                let _ = handle.with_state(|state| build.step(&world, state));
                if let Some(changed) = build.take_changed() {
                    *conditions = changed;
                    self.log_conditions(&changed, &world);
                }
            }
            if ticked != *last_tick_ok {
                if ticked {
                    curia::info!("tick recovered", {});
                } else {
                    curia::warn!("tick failed", {});
                }
                *last_tick_ok = ticked;
            }
            if handle.with_state(|state| state.take_picker_request()).unwrap_or(false) {
                picker.request();
            }
            // between ticks the core's answer changes only inside the tick, so the value read here holds until the next
            let unwanted = handle.with_state(|state| state.pause_screen_unwanted()).unwrap_or(false);
            self.pause_unwanted.store(unwanted, Ordering::SeqCst);
            let key_held = self.flags.use_key() && !menu_open;
            let (command, counts) = match self.driver.lock() {
                Ok(mut driver) => {
                    let core: &mut dyn UseCore = handle;
                    (driver.tick(&self.flags, Some(core), player.as_ref(), key_held, now), driver.counts())
                }
                Err(_) => (None, [0; 36]),
            };
            (build.take_commands(), command, counts)
        };
        self.send_commands(at, epoch, commands);
        if let (Some(command), Some(player)) = (command, player.as_ref()) {
            let pick = self.pick_block(&command);
            UseCycle::carry_out(&self.core, &self.driver, player, &client.hud(), &command, pick, &counts);
        }
        #[cfg(feature = "gate")]
        self.gate_tick(at, epoch);
    }

    // starts the core on the link once the manager's tables are there
    fn connect(&self, core: &mut CoreState) {
        if core.handle.is_some() {
            return;
        }
        let state = core.link.as_ref().map(|link| link.state());
        match state {
            Some(LinkState::Connected) => {
                let Some(link) = core.link.take() else { return };
                let handle = StratumHandle::new(link, Arc::clone(&core.shared), &self.data_dir, self.ctx.input());
                *self.ghost.lock().unwrap_or_else(PoisonError::into_inner) = Some(Arc::clone(handle.ghost()));
                core.handle = Some(handle);
                self.log.info("stratum linked", &[("data_dir", &self.data_dir.display().to_string())]);
            }
            Some(LinkState::Gone) if !core.gone_logged => {
                core.gone_logged = true;
                self.log.error("the mod manager is gone before stratum linked; the mod stays off", &[]);
            }
            _ => {}
        }
    }

    // one line per change of the build conditions: the four rules in order, then the raw game values they come from
    fn log_conditions(&self, conditions: &BuildConditions, facts: &dyn BuildFacts) {
        let text = |value: bool| if value { "true" } else { "false" };
        let in_world = facts.in_world();
        let raw = |value: String| if in_world { value } else { "not in a world".to_string() };
        let connection = raw(format!("{:?}", facts.connection_kind()));
        let realm = raw(facts.in_realm().to_string());
        let game_type = raw(format!("{:?}", facts.game_type()));
        let permission = raw(format!("{:?}", facts.permission_level()));
        let commands = raw(format!("{:?}", facts.commands_enabled()));
        self.log.info(
            "build conditions",
            &[
                ("local", text(conditions.local)),
                ("cheats", text(conditions.cheats)),
                ("operator", text(conditions.operator)),
                ("creative", text(conditions.creative)),
                ("in_world", text(in_world)),
                ("connection", &connection),
                ("in_realm", &realm),
                ("game_type", &game_type),
                ("permission_level", &permission),
                ("commands_enabled", &commands),
            ],
        );
    }

    // sends the build's commands with no lock held; a command that is not sent stops the build
    fn send_commands(&self, at: &EventCtx, epoch: u64, commands: Vec<String>) {
        // no command: the executor kept its buffer, so the core's lock is not taken again
        if commands.is_empty() {
            return;
        }
        let sent = match at.client() {
            Some(client) => {
                let mut world = ApiBuildWorld::new(client, epoch, self.ctx.blocks());
                BuildExecutor::send(&mut world, &commands)
            }
            None => false,
        };
        let mut core = self.core.lock().unwrap_or_else(PoisonError::into_inner);
        if !sent && let Some(handle) = core.handle.as_ref() {
            let _ = handle.with_state(|state| state.send_failed());
        }
        core.build.give_back(commands);
    }

    // the block of the palette id a Pick names, from the ghost palette of the joined world
    fn pick_block(&self, command: &UseCommand) -> Option<amml_api::BlockHandle> {
        let UseCommand::Pick { palette_id, .. } = command else { return None };
        self.renderer.lock().ok()?.palette_block(*palette_id)
    }

    // `UsePressed`, before the game: true cancels the game's handling of the press. With the core or the driver held
    // elsewhere, the press goes to the game
    pub fn use_pressed(&self, at: &EventCtx) -> bool {
        let Some(client) = at.client() else { return false };
        let player = client.local_player();
        let now = self.now();
        let menu_open = client.showing_menu() == Some(true);
        let key_held = self.flags.use_key() && !menu_open;
        let hud = client.hud();
        UseCycle::press(&self.core, &self.driver, &self.flags, player.as_ref(), &hud, key_held, now, |command| {
            self.pick_block(command)
        })
    }

    // `BuildBlock`, before the game's build, on the game or the server thread: true skips the game's own build of the
    // local player while the hold blocks the use. Reads only easy place's flags
    pub fn blocks_game_build(&self, event: &BuildBlock) -> bool {
        let own = event.header().origin == AmmlOrigin::THIS_MOD;
        self.flags.blocks_game_build(event.is_local_player(), own)
    }

    // `InventoryRequestDone`
    pub fn request_done(&self, id: RequestId, outcome: RequestOutcome) {
        if let Ok(mut driver) = self.driver.lock() {
            driver.request_done(id, outcome);
        }
    }

    // `MouseInput`, after every writer: the real right button is the use key. An injected event is not the player's,
    // and a down that a lease blocks or another mod cancelled never reached the game; an up always ends the hold
    pub fn mouse(&self, event: &MouseInput) {
        if event.is_injected() {
            return;
        }
        if let MouseAction::Button { button: MouseButton::Right, down } = event.action() {
            if down && event.is_cancelled() {
                return;
            }
            self.flags.set_use_key(down);
        }
    }

    // whether the real use key is down, as the input events left it
    pub fn use_key(&self) -> bool {
        self.flags.use_key()
    }

    // `WindowFocus`: a loss counts as a use key up
    pub fn focus(&self, gained: bool) {
        if !gained {
            self.flags.set_use_key(false);
        }
    }

    // `AppFocusLost`: true while the library keeps itself open through the file picker and the game must not open its
    // pause screen, as the last tick found
    pub fn pause_screen_unwanted(&self) -> bool {
        self.pause_unwanted.load(Ordering::SeqCst)
    }

    // `ServerJoinStarting`: the address as the player typed it (the unresolved URL, else the server name, else the
    // host), which names a server world the same way across its IP changes
    pub fn server_join_starting(&self, url: Option<&str>, name: Option<&str>, host: Option<&str>, port: u16) {
        let typed = [url, name, host].into_iter().flatten().find(|text| !text.is_empty());
        if let Ok(mut slot) = self.typed_server.lock() {
            *slot = typed.map(|text| (text.to_string(), port));
        }
    }

    // `WorldJoined`
    pub fn world_joined(&self, epoch: u64) {
        self.log.info("world joined", &[("epoch", &epoch.to_string())]);
    }

    // `WorldLeft`: the loader has taken back every mesh, group, ghost and handle of the world; the use key and the hold
    // end
    pub fn world_left(&self) {
        self.flags.set_use_key(false);
        if let Ok(mut typed) = self.typed_server.lock() {
            *typed = None;
        }
        if let Ok(mut driver) = self.driver.lock() {
            driver.reset(&self.flags);
        }
        let entities = self.ctx.block_entities();
        let _meshes = self.renderer.lock().map(|mut renderer| renderer.world_left(&entities)).unwrap_or(0);
        #[cfg(feature = "gate")]
        if let Ok(mut gate) = self.gate.lock() {
            gate.world_left(_meshes);
        }
        self.block_changes.clear();
    }

    // `DimensionChanging`: the ghost joins the new dimension at the next frame
    pub fn dimension_changing(&self) {
        if let Ok(mut renderer) = self.renderer.lock() {
            renderer.dimension_changing();
        }
    }

    // `WorldRendered`: the gate's mesh, then the ghost
    pub fn world_rendered(&self, event: &WorldRendered, at: &EventCtx) {
        #[cfg(feature = "gate")]
        if let Ok(mut gate) = self.gate.try_lock() {
            gate.world_rendered(event);
        }
        let Some(ghost) = self.ghost.lock().ok().and_then(|ghost| ghost.clone()) else { return };
        let Ok(mut renderer) = self.renderer.try_lock() else { return };
        renderer.frame(event, at.epoch.0, &self.ctx.blocks(), &self.ctx.block_entities(), &ghost);
    }

    // `BlockEntityPass`: the gate's ghosts, then the ghost's block-entity cells
    pub fn block_entity_pass(&self, event: &BlockEntityPass, at: &EventCtx) {
        #[cfg(feature = "gate")]
        if let Ok(mut gate) = self.gate.try_lock() {
            gate.block_entity_pass(event);
        }
        let Ok(mut renderer) = self.renderer.try_lock() else { return };
        renderer.block_entity_pass(event, at.epoch.0, &self.ctx.block_entities());
    }

    // `BlockChanged`: each position goes to the core, which rebuilds the ghost section that holds it; the block on
    // layer 0 goes too, so the core can check the placements that wait for the server against it (layer 1 holds the
    // water of a waterlogged block). A change that comes while the core is held waits in the queue for the next tick
    pub fn block_changed(&self, event: &BlockChanged) {
        let pos = event.pos();
        let position = [pos.x, pos.y, pos.z];
        let time = self.now();
        let name = if event.layer() == 0 {
            event.new_state(&self.ctx).and_then(|block| self.ctx.blocks().name(block))
        } else {
            None
        };
        #[cfg(feature = "gate")]
        if let Ok(mut gate) = self.gate.try_lock() {
            gate.block_changed(pos, name.as_deref());
        }
        match self.core.try_lock() {
            Ok(core) => {
                if let Some(handle) = core.handle.as_ref() {
                    let _ = handle.with_state(|state| {
                        state.block_changed(position);
                        if let Some(name) = &name {
                            state.block_seen(position, name.as_bytes(), time);
                        }
                    });
                }
            }
            Err(_) => {
                let seen = name.map(|name| (name, time));
                self.block_changes.push(QueuedChange { position, seen });
            }
        }
    }

    // a queued block change, as `block_changed` hands it to the core
    fn apply_change(handle: &StratumHandle, change: QueuedChange) {
        let _ = handle.with_state(|state| {
            state.block_changed(change.position);
            if let Some((name, time)) = &change.seen {
                state.block_seen(change.position, name.as_bytes(), *time);
            }
        });
    }

    // with the gate, after the tick with no lock held: its lookups and, with every build condition true, its probe,
    // scan and command
    #[cfg(feature = "gate")]
    fn gate_tick(&self, at: &EventCtx, epoch: u64) {
        let Some(client) = at.client() else { return };
        let player = client.local_player();
        let Some(position) = player.as_ref().and_then(|player| player.position()) else { return };
        let feet = PlayerSpot::feet_of([position.x, position.y, position.z]);
        let below = amml_api::BlockPos { x: feet[0], y: feet[1] - 1, z: feet[2] };
        let ground = player
            .as_ref()
            .and_then(|player| player.block_source())
            .and_then(|source| source.block(below).map(|block| (block.name(), block.handle())));
        let conditions = self.core.lock().map(|core| core.conditions).unwrap_or_default();
        let blocks = self.ctx.blocks();
        let entities = self.ctx.block_entities();
        let world = ApiBuildWorld::new(client, epoch, blocks);
        let command = match self.gate.lock() {
            Ok(mut gate) => {
                gate.tick(self.now(), feet, ground, &blocks, &entities, Some((&conditions, &world, &world)))
            }
            Err(_) => None,
        };
        let Some(command) = command else { return };
        let sent = at.client().is_some_and(|client| client.send_command(&command).is_ok());
        if let Ok(mut gate) = self.gate.lock() {
            gate.command_sent(sent);
        }
    }

    // disable, after the loader removed the listeners: the hold ends, the picker closes and its worker is joined, and
    // the core saves, unregisters and stops. The loader frees the meshes, groups and ghosts of the mod itself
    pub fn stop(&self) {
        self.flags.set_use_key(false);
        self.driver.lock().unwrap_or_else(PoisonError::into_inner).reset(&self.flags);
        self.pause_unwanted.store(false, Ordering::SeqCst);
        *self.ghost.lock().unwrap_or_else(PoisonError::into_inner) = None;
        self.renderer.lock().unwrap_or_else(PoisonError::into_inner).reset(&self.ctx.block_entities());
        let mut core = self.core.lock().unwrap_or_else(PoisonError::into_inner);
        core.picker.stop();
        core.frame.reset();
        self.block_changes.clear();
        core.link = None;
        if let Some(handle) = core.handle.take() {
            handle.shutdown();
        }
    }
}
