use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::thread::ThreadId;
use std::time::Instant;

use amm_client::link::ApiTable;
use amml_game::{
    Block, ClientInstance, ContainerClosePacketLayout, ContainerOpenPacketLayout, GameEntries, GameHeap, HeldClient,
    ItemStackResponsePacketLayout, StopLatch,
};
use amml_sdk::{HookHandle, ModDirs, ModRuntime};

use crate::easy_place::UseHooks;
use crate::game::ClientWorld;
use crate::ghost::BlockChangeListener;
use crate::services::{BlockChangeHub, BlockChangeQueue, RendererSlot, SharedGhost, StratumHandle, WorldFolder};
#[cfg(feature = "gate")]
use crate::services::{GatePlacer, GateRequest, GateRunner};
use crate::traits::UseCore;
use crate::types::{
    BlockChange, CoreState, GhostCalls, LinkState, ListenerSlots, PacketCalls, PathList, PlayerCalls, QueuedChange,
    StratumEntry, UseCalls,
};

// what the detours share: the stop latch, the client the tick saw last, the entries, the folders, the core with its
// frame reader, picker and block listener, the ghost renderer, easy place and the gate. The tick, the block changes
// and the render run on the game's main thread; the focus loss reads only an atomic the tick writes, so it takes no
// lock on whatever thread it runs; the build hook also runs on the local world's server thread and reads only easy
// place's atomics; disable may run on another thread and waits for the locks
pub struct StratumRuntime {
    latch: StopLatch,
    client: HeldClient,
    entries: OnceLock<GameEntries>,
    dirs: ModDirs,
    core: Mutex<CoreState>,
    // the ghost frames and reports of the running core, for the render thread
    ghost: Mutex<Option<Arc<SharedGhost>>>,
    renderer: RendererSlot,
    // the block changes that came while the core was held, for the next tick
    block_changes: BlockChangeQueue,
    // easy place: its use lock and the signals its hooks read
    uses: UseHooks,
    #[cfg(feature = "gate")]
    gate: Mutex<Option<GateRunner>>,
    // the gate's item stack request, on the game thread; the packet hooks take it with try_lock
    #[cfg(feature = "gate")]
    request: Mutex<Option<GateRequest>>,
    // whether the request has ended, for the render thread's frames line
    #[cfg(feature = "gate")]
    request_settled: AtomicBool,
    hooks: Mutex<Vec<HookHandle>>,
    // the core's answer at the end of the last tick: the library keeps itself open through the file picker
    pause_unwanted: AtomicBool,
    // the thread the client update runs on
    game_thread: Mutex<Option<ThreadId>>,
    // the start of the steady clock the frame status carries
    epoch: Instant,
}

static RUNTIME: OnceLock<StratumRuntime> = OnceLock::new();

// where the block listener's thunks send each change
static BLOCK_HUB: BlockChangeHub = BlockChangeHub::new(StratumRuntime::block_changed);

impl StratumRuntime {
    // the detours: client update, focus loss, world storage made and shut down, and render level; then easy place's
    // screen controller destructor, HUD tick, item stack response, container open and container close handlers,
    // game mode build and build action
    pub const HOOK_CAPACITY: usize = 12;

    pub fn new(dirs: ModDirs) -> Self {
        Self {
            latch: StopLatch::new(),
            client: HeldClient::new(),
            entries: OnceLock::new(),
            dirs,
            core: Mutex::new(CoreState::new()),
            ghost: Mutex::new(None),
            renderer: RendererSlot::new(),
            block_changes: BlockChangeQueue::new(),
            uses: UseHooks::new(),
            #[cfg(feature = "gate")]
            gate: Mutex::new(None),
            #[cfg(feature = "gate")]
            request: Mutex::new(None),
            #[cfg(feature = "gate")]
            request_settled: AtomicBool::new(false),
            hooks: Mutex::new(Vec::with_capacity(Self::HOOK_CAPACITY)),
            pause_unwanted: AtomicBool::new(false),
            game_thread: Mutex::new(None),
            epoch: Instant::now(),
        }
    }

    // the runtime of this DLL; the first call wins
    pub fn install(dirs: ModDirs) -> &'static StratumRuntime {
        RUNTIME.get_or_init(|| Self::new(dirs))
    }

    pub fn get() -> Option<&'static StratumRuntime> {
        RUNTIME.get()
    }

    // raised by disable and by a panic in a hook body
    pub fn latch(&self) -> &StopLatch {
        &self.latch
    }

    // the entries the first enable kept
    pub fn entries(&self) -> Option<&GameEntries> {
        self.entries.get()
    }

    // the mod's folders, as the loader gave them at load
    pub fn dirs(&self) -> &ModDirs {
        &self.dirs
    }

    /// The game heap's allocator, through the loader; null before load or when the heap gives no block.
    ///
    /// # Safety
    /// None beyond the loader's: any size is accepted.
    #[allow(unsafe_code)]
    pub unsafe extern "C" fn heap_new(size: usize) -> *mut u8 {
        ModRuntime::get().map_or(std::ptr::null_mut(), |runtime| runtime.api().heap_new(size))
    }

    /// The game heap's operator delete, through the loader.
    ///
    /// # Safety
    /// `ptr` must be a block of the game heap of `size` bytes that is not used again.
    #[allow(unsafe_code)]
    pub unsafe extern "C" fn heap_delete(ptr: *mut u8, size: usize) {
        if let Some(runtime) = ModRuntime::get() {
            // SAFETY: the caller guarantees a block of the game heap with the size it was allocated with
            unsafe { runtime.api().heap_delete(ptr, size) };
        }
    }

    // enable: keeps the entries, makes the ghost renderer (or has the next frame reset the one of the last enable,
    // clearing a lock a render panic poisoned) and, with the gate, a gate with the calls and the sample placer; the
    // latch goes down
    pub fn start(&self, entries: GameEntries, calls: &GhostCalls) {
        let _ = self.entries.set(entries);
        self.renderer.install(calls);
        self.start_easy_place();
        #[cfg(feature = "gate")]
        {
            if self.gate.is_poisoned() {
                curia::warn!("the gate lock was poisoned by a render panic; it is cleared", {});
                self.gate.clear_poison();
            }
            let mut slot = self.gate.lock().unwrap_or_else(PoisonError::into_inner);
            if slot.is_none() {
                *slot = Some(GateRunner::new(calls));
            }
            self.core.lock().unwrap_or_else(PoisonError::into_inner).placer =
                Some(GatePlacer::new(&GatePlacer::samples_folder()));
            let calls = self.entries.get().map(|entries| (PacketCalls::new(entries), PlayerCalls::new(entries)));
            let request = match calls {
                Some((Ok(packets), Ok(players))) => Some(GateRequest::new(packets, players)),
                Some((Err(error), _) | (_, Err(error))) => {
                    Self::gate_log(&format!("gate request not made ({error})"));
                    None
                }
                None => None,
            };
            self.request_settled.store(request.is_none(), Ordering::SeqCst);
            *self.request.lock().unwrap_or_else(PoisonError::into_inner) = request;
        }
        self.latch.lower();
    }

    // enable: the use driver for the calls
    fn start_easy_place(&self) {
        let Some(entries) = self.entries.get() else { return };
        match (PacketCalls::new(entries), PlayerCalls::new(entries), UseCalls::new(entries)) {
            (Ok(packets), Ok(players), Ok(calls)) => self.uses.start(&packets, &players, &calls),
            (Err(error), _, _) | (_, Err(error), _) | (_, _, Err(error)) => {
                curia::warn!("easy place is off, its calls were not made", { error: error.to_string() });
            }
        }
    }

    // enable: registers with the mod manager and starts the core now; when the manager is not there yet, the first
    // tick tries once more. A lock poisoned by a panic before the last disable is cleared: `stop` dropped the state
    // the panic left half done
    pub fn start_core(&self) {
        if self.core.is_poisoned() {
            self.core.clear_poison();
        }
        let mut core = self.core.lock().unwrap_or_else(PoisonError::into_inner);
        if core.handle.is_some() {
            return;
        }
        core.link = LinkState::Waiting;
        self.attach(&mut core, false);
    }

    // keeps an installed hook; false, keeping nothing, when HOOK_CAPACITY hooks are already kept
    pub fn keep_hook(&self, hook: HookHandle) -> bool {
        let mut hooks = self.hooks.lock().unwrap_or_else(PoisonError::into_inner);
        if hooks.len() >= Self::HOOK_CAPACITY {
            return false;
        }
        hooks.push(hook);
        true
    }

    // the installed hooks, newest first, leaving none behind
    pub fn take_hooks(&self) -> Vec<HookHandle> {
        let mut hooks = std::mem::take(&mut *self.hooks.lock().unwrap_or_else(PoisonError::into_inner));
        hooks.reverse();
        hooks
    }

    // the body of the client update detour, after the game's own update: reads the frame status, keeps the block
    // listener on the client region, hands a finished pick to the core, runs the core's tick, then opens the picker
    // when the library asked for it
    #[allow(unsafe_code)]
    pub fn tick(&self, client: NonNull<c_void>) {
        self.client.set(client.as_ptr());
        self.mark_game_thread();
        let Some(entries) = self.entries.get() else { return };
        let Ok(mut core) = self.core.lock() else { return };
        if core.link == LinkState::Waiting {
            self.attach(&mut core, true);
        }
        // SAFETY: `client` is the live ClientInstance of the running update
        let game_client = unsafe { ClientInstance::from_raw(client.as_ptr(), entries) };
        self.sync_listener(&mut core, entries, game_client.as_ref());
        #[cfg(feature = "gate")]
        self.tick_request(game_client.as_ref());
        #[cfg(feature = "gate")]
        let yaw = game_client.as_ref().and_then(ClientInstance::local_player).and_then(|player| player.pose());
        #[cfg(feature = "gate")]
        let yaw = yaw.map(|pose| pose.yaw);
        #[cfg(feature = "gate")]
        let CoreState { handle, frame, picker, last_tick_ok, placer, .. } = &mut *core;
        #[cfg(not(feature = "gate"))]
        let CoreState { handle, frame, picker, last_tick_ok, .. } = &mut *core;
        let Some(handle) = handle.as_ref() else { return };
        self.block_changes.drain(|change| Self::apply_change(handle, change));
        let dropped = self.block_changes.take_dropped();
        if dropped > 0 {
            curia::warn!("block changes dropped, the block change queue was full", { dropped: dropped });
        }
        // SAFETY: `client` is the live ClientInstance of the running update
        let Some(source) = (unsafe { ClientWorld::from_raw(client.as_ptr(), entries, WorldFolder::global()) }) else {
            return;
        };
        let status = frame.read(&source, self.epoch.elapsed().as_secs_f64());
        if let Some(answer) = picker.deliver() {
            let files = PathList::new(answer.paths).into_paths();
            if handle.with_state(|state| state.files_picked(files)).is_none() {
                curia::warn!("picked files not handed to the library", {});
            }
        }
        #[cfg(feature = "gate")]
        let ahead = placer.as_mut().and_then(|placer| placer.feet_for_tick(status, yaw, handle.shared()));
        #[cfg(not(feature = "gate"))]
        let ahead: Option<[i32; 3]> = None;
        let ticked = match ahead {
            Some(feet) => {
                let mut moved = status.clone();
                moved.feet = feet;
                handle.with_state(|state| state.tick(&moved)).is_some()
            }
            None => handle.with_state(|state| state.tick(status)).is_some(),
        };
        #[cfg(feature = "gate")]
        if let Some(placer) = placer.as_mut() {
            placer.after_tick(handle.shared());
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
        if let Some(game_client) = game_client.as_ref() {
            let frame = UseHooks::frame(game_client, self.epoch.elapsed().as_secs_f64());
            let mut core = handle;
            // SAFETY: the frame's client, player and region are the live objects of the running update
            unsafe {
                self.uses.tick(Some(&mut core as &mut dyn UseCore), &frame, &mut |id| self.renderer.palette_block(id))
            };
        }
    }

    // the body of the build action detour, before the game: true cancels the game's handling of the use press. With
    // the core held elsewhere, the press goes to the game
    #[allow(unsafe_code)]
    pub fn on_build_action(&self, client: usize) -> bool {
        let Some(entries) = self.entries.get() else { return false };
        // SAFETY: the game passes its live client
        let Some(game_client) = (unsafe { ClientInstance::from_raw(client as *mut c_void, entries) }) else {
            return false;
        };
        let frame = UseHooks::frame(&game_client, self.epoch.elapsed().as_secs_f64());
        let Ok(core) = self.core.try_lock() else { return false };
        let mut handle = core.handle.as_ref();
        let use_core = handle.as_mut().map(|handle| handle as &mut dyn UseCore);
        // SAFETY: the frame's client, player and region are the live objects of the running press
        unsafe { self.uses.press(use_core, &frame, &mut |id| self.renderer.palette_block(id)) }
    }

    // the body of the build block detour, before the game: true skips the game's own build. Any thread
    #[allow(unsafe_code)]
    pub fn blocks_game_build(&self, game_mode: usize, is_sim_tick: bool) -> bool {
        // SAFETY: the game passes the live GameMode of the running build
        unsafe { self.uses.blocks_game_build(game_mode as *mut c_void, is_sim_tick) }
    }

    // the body of the HUD tick detour, before the game's tick
    pub fn hud_tick(&self, hud: usize) {
        self.uses.hud_tick(hud, self.epoch.elapsed().as_secs_f64());
    }

    // the body of the screen controller destructor detour, before the game's destructor
    pub fn screen_gone(&self, controller: usize) {
        self.uses.screen_gone(controller);
    }

    // the body of the focus loss detour: true while the library keeps itself open through the file picker and the
    // game must not open its pause screen, as the last tick found. False while stopped or with no core
    pub fn pause_screen_unwanted(&self) -> bool {
        !self.latch.raised() && self.pause_unwanted.load(Ordering::SeqCst)
    }

    // the body of the render level detour, after the game's own level pass: the gate's mesh, then the ghost
    #[allow(unsafe_code)]
    pub fn after_render_level(&self, renderer: usize, screen: usize, frame: usize) {
        if self.latch.raised() {
            return;
        }
        let Some(entries) = self.entries.get() else { return };
        let client = self.client.get(&self.latch);
        // the render holds each lock alone unless a disable runs
        #[cfg(feature = "gate")]
        if let Ok(mut gate) = self.gate.try_lock()
            && let Some(gate) = gate.as_mut()
        {
            let mut log = |line: &str| Self::gate_log(line);
            gate.hold_frames_line(!self.request_settled.load(Ordering::SeqCst));
            // SAFETY: the arguments of the running renderLevel call, live for it
            unsafe {
                gate.after_render_level(
                    entries,
                    client,
                    renderer as *mut _,
                    screen as *mut _,
                    frame as *const _,
                    &mut log,
                )
            };
        }
        let Some(ghost) = self.ghost.try_lock().ok().and_then(|ghost| ghost.clone()) else { return };
        self.renderer.frame(|ghost_renderer| {
            // SAFETY: the arguments of the running renderLevel call, live for it
            unsafe {
                ghost_renderer.after_render_level(
                    entries,
                    client,
                    renderer as *mut _,
                    screen as *mut _,
                    frame as *const _,
                    &ghost,
                )
            };
        });
    }

    // the block listener's sink, on the game thread: each position goes to the core, which rebuilds the ghost section
    // that holds it; the block on layer 0 goes too, so the core can check the placements that wait for the server
    // against it (layer 1 holds the water of a waterlogged block). A change that comes while the core is held waits
    // in the queue, with a copy of the name, for the next tick
    #[allow(unsafe_code)]
    pub fn block_changed(change: &BlockChange) {
        let Some(runtime) = Self::get() else { return };
        if runtime.latch.raised() {
            return;
        }
        let Some(entries) = runtime.entries.get() else { return };
        let position = [change.pos.x, change.pos.y, change.pos.z];
        let time = runtime.epoch.elapsed().as_secs_f64();
        // SAFETY: the game passes the new block, live for the call
        let block = unsafe { Block::from_raw(change.block.cast_mut(), entries) };
        let name = block.as_ref().filter(|_| change.layer == 0).and_then(Block::type_name).map(|name| name.as_bytes());
        match runtime.core.try_lock() {
            Ok(core) => {
                if let Some(handle) = core.handle.as_ref() {
                    let _ = handle.with_state(|state| {
                        state.block_changed(position);
                        if let Some(name) = name {
                            state.block_seen(position, name, time);
                        }
                    });
                }
            }
            Err(_) => {
                let seen = name.map(|name| (name.to_vec(), time));
                runtime.block_changes.push(QueuedChange { position, seen });
            }
        }
    }

    // a queued block change, as `block_changed` hands it to the core
    fn apply_change(handle: &StratumHandle, change: QueuedChange) {
        let _ = handle.with_state(|state| {
            state.block_changed(change.position);
            if let Some((name, time)) = &change.seen {
                state.block_seen(change.position, name, *time);
            }
        });
    }

    // with the gate, on the game thread: the item stack request's step while a local player is in an active world
    #[cfg(feature = "gate")]
    fn tick_request(&self, client: Option<&ClientInstance<'_>>) {
        let Some(client) = client.filter(|client| client.is_world_active()) else { return };
        let Some(player) = client.local_player() else { return };
        let Ok(mut request) = self.request.try_lock() else { return };
        let Some(request) = request.as_mut() else { return };
        let now = self.epoch.elapsed().as_secs_f64();
        // SAFETY: the live client of the running update and its live local player
        #[allow(unsafe_code)]
        unsafe {
            request.tick(now, client.as_ptr(), player.as_ptr(), &mut |line| Self::gate_log(line))
        };
        self.request_settled.store(request.settled(), Ordering::SeqCst);
    }

    /// The body of the ContainerOpen handler detour: true hides the packet from the game. `packet` is the handler's
    /// `ContainerOpenPacket const&`. Easy place answers first, then the gate.
    #[allow(unsafe_code)]
    pub fn on_container_open(&self, packet: usize) -> bool {
        // SAFETY: the handler's live ContainerOpenPacket, laid out as the mirror
        let Some(packet) = (unsafe { (packet as *const ContainerOpenPacketLayout).as_ref() }) else { return false };
        let now = self.epoch.elapsed().as_secs_f64();
        if self.uses.on_container_open(now, packet) {
            return true;
        }
        #[cfg(feature = "gate")]
        if self.gate_container_open(now, packet) {
            return true;
        }
        false
    }

    /// The body of the ContainerClose handler detour: true hides the packet from the game. Easy place answers first,
    /// then the gate.
    #[allow(unsafe_code)]
    pub fn on_container_close(&self, packet: usize) -> bool {
        // SAFETY: the handler's live ContainerClosePacket, laid out as the mirror
        let Some(packet) = (unsafe { (packet as *const ContainerClosePacketLayout).as_ref() }) else { return false };
        let now = self.epoch.elapsed().as_secs_f64();
        if self.uses.on_container_close(now, packet) {
            return true;
        }
        #[cfg(feature = "gate")]
        if self.gate_container_close(now, packet) {
            return true;
        }
        false
    }

    /// The body of the ItemStackResponse handler detour, before the game's own handler: easy place reads it, then the
    /// gate.
    #[allow(unsafe_code)]
    pub fn on_item_stack_response(&self, packet: usize) {
        // SAFETY: the handler's live ItemStackResponsePacket, laid out as the mirror
        let Some(packet) = (unsafe { (packet as *const ItemStackResponsePacketLayout).as_ref() }) else { return };
        self.uses.on_response(packet);
        #[cfg(feature = "gate")]
        self.gate_item_stack_response(packet);
    }

    // with the gate: its ContainerOpen; true hides the packet
    #[cfg(feature = "gate")]
    #[allow(unsafe_code)]
    fn gate_container_open(&self, now: f64, packet: &ContainerOpenPacketLayout) -> bool {
        let Ok(mut request) = self.request.try_lock() else { return false };
        let Some(request) = request.as_mut() else { return false };
        let client = self.client.get(&self.latch).map_or(std::ptr::null_mut(), NonNull::as_ptr);
        // SAFETY: the client the last tick saw, live while the game runs, or null
        unsafe { request.on_container_open(now, packet, client, &mut |line| Self::gate_log(line)) }
    }

    // with the gate: its ContainerClose; true hides the packet
    #[cfg(feature = "gate")]
    fn gate_container_close(&self, now: f64, packet: &ContainerClosePacketLayout) -> bool {
        let Ok(mut request) = self.request.try_lock() else { return false };
        let Some(request) = request.as_mut() else { return false };
        request.on_container_close(now, packet, &mut |line| Self::gate_log(line))
    }

    // with the gate: its item stack response
    #[cfg(feature = "gate")]
    #[allow(unsafe_code)]
    fn gate_item_stack_response(&self, packet: &ItemStackResponsePacketLayout) {
        let Some(entries) = self.entries.get() else { return };
        let Ok(mut request) = self.request.try_lock() else { return };
        let Some(request) = request.as_mut() else { return };
        let client = self.client.get(&self.latch);
        // SAFETY: the client the last tick saw, live while the game runs
        let client = client.and_then(|client| unsafe { ClientInstance::from_raw(client.as_ptr(), entries) });
        let player = client.as_ref().and_then(ClientInstance::local_player);
        let player = player.map_or(std::ptr::null_mut(), |player| player.as_ptr());
        let now = self.epoch.elapsed().as_secs_f64();
        // SAFETY: the live local player or null
        unsafe { request.on_response(now, packet, player, &mut |line| Self::gate_log(line)) };
        self.request_settled.store(request.settled(), Ordering::SeqCst);
    }

    // one gate line in the mod's log
    #[cfg(feature = "gate")]
    fn gate_log(line: &str) {
        if let Some(runtime) = ModRuntime::get() {
            runtime.logger().info(line);
        }
    }

    // disable, after the hooks are gone: the latch stays up, easy place ends its hold and undoes its request in flight,
    // and the client is forgotten; then the picker closes and
    // its worker is joined, the block listener leaves its source, the world key is reset, and the core saves,
    // unregisters and stops. The meshes stay with the renderer: they belong to the game's render device and are
    // destroyed on the render thread only, at the next enable's first frame
    pub fn stop(&self) {
        self.latch.raise();
        self.uses.reset();
        self.client.clear();
        self.pause_unwanted.store(false, Ordering::SeqCst);
        *self.ghost.lock().unwrap_or_else(PoisonError::into_inner) = None;
        let mut core = self.core.lock().unwrap_or_else(PoisonError::into_inner);
        core.picker.stop();
        core.frame.reset();
        self.release_listener(&mut core);
        self.block_changes.clear();
        #[cfg(feature = "gate")]
        {
            core.placer = None;
            *self.request.lock().unwrap_or_else(PoisonError::into_inner) = None;
        }
        WorldFolder::global().clear();
        if let Some(handle) = core.handle.take() {
            handle.shutdown();
        }
        core.link = LinkState::Off;
    }

    // `path` made absolute against the working folder; the manager and the library need absolute paths
    fn absolute(path: &Path) -> PathBuf {
        std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf())
    }

    fn attach(&self, core: &mut CoreState, last_try: bool) {
        let config_dir = Self::absolute(&self.dirs.config_dir);
        let data_dir = Self::absolute(&self.dirs.data_dir);
        let connected = ApiTable::resolve()
            .map_err(|error| error.to_string())
            .and_then(|table| StratumHandle::connect(table, &config_dir, &data_dir).map_err(|error| error.to_string()));
        match connected {
            Ok(handle) => {
                *self.ghost.lock().unwrap_or_else(PoisonError::into_inner) = Some(Arc::clone(handle.ghost()));
                core.handle = Some(handle);
                core.link = LinkState::Attached;
                curia::info!("stratum enabled", {
                    config_dir: config_dir.display().to_string(),
                    data_dir: data_dir.display().to_string()
                });
            }
            Err(error) if last_try => {
                core.link = LinkState::Off;
                if let Some(runtime) = ModRuntime::get() {
                    runtime.logger().error(&format!(
                        "@amml/modmanager is not loaded or refused stratum ({error}); the mod stays off"
                    ));
                }
            }
            Err(_) => {}
        }
    }

    fn mark_game_thread(&self) {
        let mut game_thread = self.game_thread.lock().unwrap_or_else(PoisonError::into_inner);
        if game_thread.is_none() {
            *game_thread = Some(std::thread::current().id());
        }
    }

    fn on_game_thread(&self) -> bool {
        *self.game_thread.lock().unwrap_or_else(PoisonError::into_inner) == Some(std::thread::current().id())
    }

    // the listener's base table and slots; None when an entry is missing
    fn listener_slots(entries: &GameEntries) -> Option<(u64, ListenerSlots)> {
        let base = entries.get(StratumEntry::BLOCK_SOURCE_LISTENER_VTABLE)?;
        let slots = ListenerSlots {
            add_listener: entries.get(StratumEntry::BLOCK_SOURCE_ADD_LISTENER)?,
            remove_listener: entries.get(StratumEntry::BLOCK_SOURCE_REMOVE_LISTENER)?,
            source_destroyed: entries.get(StratumEntry::BLOCK_SOURCE_LISTENER_ON_SOURCE_DESTROYED)?,
            block_changed: entries.get(StratumEntry::BLOCK_SOURCE_LISTENER_ON_BLOCK_CHANGED)?,
        };
        Some((base, slots))
    }

    // keeps the listener on the client region while there is a local player in an active world, and off any region
    // otherwise; a region the game destroyed was forgotten by its onSourceDestroyed, so the next region is joined
    #[allow(unsafe_code)]
    fn sync_listener(&self, core: &mut CoreState, entries: &GameEntries, client: Option<&ClientInstance<'_>>) {
        let region = client
            .filter(|client| client.is_world_active() && client.local_player().is_some())
            .and_then(ClientInstance::region)
            .map(|region| region.as_ptr());
        let Some(region) = region else {
            if let Some(listener) = core.listener.as_ref() {
                listener.detach();
            }
            return;
        };
        if core.listener.is_none() && !core.listener_failed {
            let made = match (Self::listener_slots(entries), GameHeap::msvc()) {
                // SAFETY: the base is the game's 9-slot BlockSourceListener table and the heap the game's
                (Some((base, slots)), Some(heap)) => {
                    unsafe { BlockChangeListener::new(base, slots, &BLOCK_HUB, &heap) }
                        .map_err(|error| error.to_string())
                }
                _ => Err("an entry or the game heap is missing".to_string()),
            };
            match made {
                Ok(listener) => core.listener = Some(listener),
                Err(error) => {
                    core.listener_failed = true;
                    curia::error!("block changes are off: the listener was not made", { error: error });
                }
            }
        }
        if let Some(listener) = core.listener.as_ref()
            && listener.attached() != region
        {
            // SAFETY: the client's live region, whose vtable the listener slots were resolved for
            unsafe { listener.attach(region) };
        }
    }

    // on the game thread the listener leaves its source and is freed; off it, the game may be calling it through the
    // source's listener list right now, so it stays registered and allocated, its thunks stopped by the latch, and the
    // next enable's ticks use it again
    #[allow(unsafe_code)]
    fn release_listener(&self, core: &mut CoreState) {
        let Some(listener) = core.listener.take() else { return };
        if !self.on_game_thread() && listener.is_attached() {
            curia::info!("disable ran off the game thread while a source held the block listener; it stays", {});
            core.listener = Some(listener);
            return;
        }
        listener.detach();
        let Some(heap) = GameHeap::msvc() else {
            core.listener = Some(listener);
            return;
        };
        // SAFETY: the listener was allocated on the game heap; it was just removed from its source, or its source
        // was destroyed, and no hook runs any more
        if let Err(listener) = unsafe { listener.release(&heap) } {
            curia::warn!("the block listener is still held by a source and is not freed", {});
            core.listener = Some(listener);
        }
    }
}
