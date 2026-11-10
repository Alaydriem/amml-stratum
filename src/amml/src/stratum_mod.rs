use std::ffi::c_void;
use std::sync::atomic::AtomicPtr;

use amml_game::{EntryId, EntryNeed, EntryResolver, GameEntries, GameError, HookOrder};
use amml_sdk::amml_abi::AmmlLogLevel;
use amml_sdk::{AmmlMod, Api, HookHandle, ModDirs, SdkError};

use crate::detours::{
    build_action, build_block, client_update, container_close, container_open, db_storage_ctor, db_storage_shutdown,
    focus_lost, hud_tick, render_level, screen_gone, stack_response,
};
use crate::errors::GameLayoutError;
use crate::game::ClientLevel;
use crate::services::{LegacyFolders, StratumRuntime};
use crate::types::{GhostCalls, PacketCalls, PlayerCalls, StratumEntry, StratumFeature, UseCalls};

// the AMML entry point: load notes the folders and installs the runtime; enable resolves the entries, copies the old
// LeviLamina folders once, starts the core and installs the hooks; disable removes the hooks, closes the picker and
// stops the core
pub struct StratumMod;

impl StratumMod {
    // whether this build runs the in-game gate
    pub const GATE: bool = cfg!(feature = "gate");

    // every entry stratum asks for: all are required, except the optional features' entries
    pub fn wanted() -> Vec<(EntryId, EntryNeed)> {
        let required = StratumEntry::GHOST
            .iter()
            .chain(StratumEntry::PALETTE.iter())
            .chain(StratumEntry::BLOCK_CHANGES.iter())
            .chain(StratumEntry::SHARED.iter())
            .chain(StratumEntry::WORLD_KEY.iter())
            .chain(StratumEntry::STATUS.iter())
            .chain(StratumEntry::EASY_PLACE.iter())
            .map(|id| (*id, EntryNeed::Required));
        let optional = StratumFeature::OPTIONAL.iter().flat_map(|feature| {
            StratumFeature::entries(feature).iter().map(|id| (*id, EntryNeed::Optional { feature }))
        });
        required.chain(optional).collect()
    }

    // the entries, or an error when a required entry is missing
    pub fn resolve<R: EntryResolver>(resolver: &R) -> Result<GameEntries, GameError> {
        GameEntries::resolve(resolver, &Self::wanted())
    }

    // one line per feature that is off, naming it and the entry that turned it off
    pub fn off_lines(entries: &GameEntries) -> Vec<String> {
        entries.off_features().into_iter().map(|(feature, cause)| format!("{feature} is off ({cause})")).collect()
    }

    // the one line that names every feature that is off, or none
    pub fn features_line(entries: &GameEntries) -> String {
        let off: Vec<&str> = entries.off_features().into_iter().map(|(feature, _)| feature).collect();
        if off.is_empty() { "features off: none".to_string() } else { format!("features off: {}", off.join(", ")) }
    }

    // whether the easy place calls accept the entries; the use calls also check the game mode mirror against its
    // anchor. A refusal refuses the enable, as the ghost calls do
    pub fn easy_place_check(entries: &GameEntries) -> Result<(), GameLayoutError> {
        PacketCalls::new(entries).and(PlayerCalls::new(entries)).and(UseCalls::new(entries)).map(|_| ())
    }

    #[allow(unsafe_code)]
    fn hook(
        api: &Api,
        runtime: &StratumRuntime,
        target: u64,
        detour: *const c_void,
        priority: i32,
        original: &'static AtomicPtr<c_void>,
    ) -> Result<HookHandle, SdkError> {
        // SAFETY: `target` is the derived start of the game function each caller pairs with the detour written for
        // its signature (this in rcx, then the C++ parameters as the game passes them); each original slot belongs
        // to one hook only
        let hook = unsafe { api.hook(target, detour, priority, original) }?;
        if runtime.keep_hook(hook) {
            return Ok(hook);
        }
        let _ = api.unhook(hook);
        Err(SdkError::Mod(format!("more than {} hooks", StratumRuntime::HOOK_CAPACITY)))
    }

    fn remove_hooks(api: &Api, runtime: &StratumRuntime) {
        for hook in runtime.take_hooks() {
            if let Err(error) = api.unhook(hook) {
                api.log(&Self::id(), AmmlLogLevel::WARN, &format!("unhook failed: {error}"));
            }
        }
    }

    // the update, the storage and the render hooks only read after the game, so they run after every writer; the
    // focus loss may skip the game's handler, so it is a writer. The storage hooks are skipped while their feature
    // is off, and a failure to install one turns the feature off
    fn install_hooks(api: &Api, runtime: &StratumRuntime, entries: &GameEntries) -> Result<(), SdkError> {
        let get = |id: EntryId| entries.get(id).ok_or_else(|| SdkError::MissingEntry(id.as_str().to_string()));
        let update = get(EntryId::CLIENT_INSTANCE_UPDATE)?;
        let focus = get(StratumEntry::MINECRAFT_GAME_ON_APP_FOCUS_LOST)?;
        let (read, write) = (HookOrder::READ_AFTER_WRITERS, HookOrder::WRITE);
        Self::hook(
            api,
            runtime,
            update,
            client_update::detour as *const c_void,
            read,
            client_update::GUARD.original(),
        )?;
        Self::hook(api, runtime, focus, focus_lost::detour as *const c_void, write, focus_lost::GUARD.original())?;
        let storage =
            entries.get(StratumEntry::DB_STORAGE_CTOR).zip(entries.get(StratumEntry::DB_STORAGE_CHECK_SHUTDOWN_DONE));
        if let Some((ctor, shutdown)) = storage {
            let made = db_storage_ctor::detour as *const c_void;
            let done = db_storage_shutdown::detour as *const c_void;
            let installed = Self::hook(api, runtime, shutdown, done, read, db_storage_shutdown::GUARD.original())
                .and_then(|_| Self::hook(api, runtime, ctor, made, read, db_storage_ctor::GUARD.original()));
            if let Err(error) = installed {
                let line = format!("{} is off (the storage hooks: {error})", StratumFeature::WORLD_FOLDER);
                api.log(&Self::id(), AmmlLogLevel::WARN, &line);
            }
        }
        let render = get(StratumEntry::LEVEL_RENDERER_RENDER_LEVEL)?;
        let detour = render_level::detour as *const c_void;
        Self::hook(api, runtime, render, detour, read, render_level::GUARD.original())?;
        Self::install_easy_place_hooks(api, runtime, entries)
    }

    // the code pointer in `slot` of the vtable at `table`, an .rdata table that holds the slot
    #[allow(unsafe_code)]
    fn table_slot(table: u64, slot: u64) -> Result<u64, SdkError> {
        // SAFETY: each caller passes a vtable entry in .rdata and the slot entry of that class
        let target = unsafe { *((table + slot * 8) as *const u64) };
        if target == 0 {
            Err(SdkError::Mod(format!("slot {slot} of the table {table:#x} is empty")))
        } else {
            Ok(target)
        }
    }

    // easy place, in the C++ order: the screen controller destructor and the HUD tick, the item stack response,
    // container open and container close handlers of the live LegacyClientNetworkHandler (each the body its vtable
    // holds at the packet's slot), the base GameMode::buildBlock body (slot 6 of the GameMode table, which the client
    // game mode's leaf jumps to) and the build action. The HUD hooks only note a pointer, so they read after the
    // writers; the others may hide a packet or skip the game's build, so they are writers
    fn install_easy_place_hooks(api: &Api, runtime: &StratumRuntime, entries: &GameEntries) -> Result<(), SdkError> {
        let get = |id: EntryId| entries.get(id).ok_or_else(|| SdkError::MissingEntry(id.as_str().to_string()));
        let network = get(StratumEntry::LEGACY_CLIENT_NETWORK_HANDLER_VTABLE)?;
        let handler = |slot: EntryId| get(slot).and_then(|slot| Self::table_slot(network, slot));
        let hud_table = get(StratumEntry::HUD_SCREEN_CONTROLLER_VTABLE)?;
        let game_mode_table = get(StratumEntry::GAME_MODE_VTABLE)?;
        let (read, write) = (HookOrder::READ_AFTER_WRITERS, HookOrder::WRITE);
        let hooks: [(u64, *const c_void, i32, &'static AtomicPtr<c_void>); 7] = [
            (
                get(StratumEntry::SCREEN_CONTROLLER_DTOR)?,
                screen_gone::detour as *const c_void,
                read,
                screen_gone::GUARD.original(),
            ),
            (
                Self::table_slot(hud_table, get(StratumEntry::HUD_SCREEN_CONTROLLER_TICK)?)?,
                hud_tick::detour as *const c_void,
                read,
                hud_tick::GUARD.original(),
            ),
            (
                handler(StratumEntry::HANDLE_ITEM_STACK_RESPONSE)?,
                stack_response::detour as *const c_void,
                write,
                stack_response::GUARD.original(),
            ),
            (
                handler(StratumEntry::HANDLE_CONTAINER_OPEN)?,
                container_open::detour as *const c_void,
                write,
                container_open::GUARD.original(),
            ),
            (
                handler(StratumEntry::HANDLE_CONTAINER_CLOSE)?,
                container_close::detour as *const c_void,
                write,
                container_close::GUARD.original(),
            ),
            (
                Self::table_slot(game_mode_table, get(StratumEntry::GAME_MODE_BUILD_BLOCK)?)?,
                build_block::detour as *const c_void,
                write,
                build_block::GUARD.original(),
            ),
            (
                get(StratumEntry::CLIENT_INPUT_CALLBACKS_HANDLE_BUILD_ACTION)?,
                build_action::detour as *const c_void,
                write,
                build_action::GUARD.original(),
            ),
        ];
        for (target, detour, priority, original) in hooks {
            Self::hook(api, runtime, target, detour, priority, original)?;
        }
        Ok(())
    }
}

impl AmmlMod for StratumMod {
    const NAMESPACE: &'static str = "alaydriem";
    const NAME: &'static str = "stratum";

    // runs under the loader lock: only the folders are noted and the runtime is made, nothing starts a thread
    fn load(api: &Api) -> Result<(), SdkError> {
        let dirs = ModDirs::new(api, &Self::id())?;
        StratumRuntime::install(dirs);
        Ok(())
    }

    // no game call and no game heap use here: the standalone host enables before the game is initialized
    fn enable(api: &Api) -> Result<(), SdkError> {
        let runtime = StratumRuntime::get().ok_or_else(|| SdkError::Mod("load did not run".to_string()))?;
        let entries = Self::resolve(api).map_err(|error| SdkError::Mod(error.to_string()))?;
        ClientLevel::check(&entries).map_err(|error| SdkError::Mod(error.to_string()))?;
        let calls = GhostCalls::new(&entries, StratumRuntime::heap_new, StratumRuntime::heap_delete)
            .map_err(|error| SdkError::Mod(error.to_string()))?;
        Self::easy_place_check(&entries).map_err(|error| SdkError::Mod(error.to_string()))?;
        for line in &Self::off_lines(&entries) {
            api.log(&Self::id(), AmmlLogLevel::WARN, line);
        }
        api.log(&Self::id(), AmmlLogLevel::INFO, &Self::features_line(&entries));
        for (warn, line) in LegacyFolders::copy_once(runtime.dirs()).lines() {
            api.log(&Self::id(), if warn { AmmlLogLevel::WARN } else { AmmlLogLevel::INFO }, &line);
        }
        runtime.start(entries, &calls);
        runtime.start_core();
        let installed = match runtime.entries() {
            Some(entries) => Self::install_hooks(api, runtime, entries),
            None => Err(SdkError::Mod("the runtime kept no entries".to_string())),
        };
        if let Err(error) = installed {
            Self::remove_hooks(api, runtime);
            runtime.stop();
            return Err(error);
        }
        if Self::GATE {
            api.log(&Self::id(), AmmlLogLevel::INFO, "gate on: the first world frame builds one stone mesh");
        }
        Ok(())
    }

    // the hooks go first, newest first, so no new tick or frame enters stratum; then easy place undoes its request in
    // flight, the picker closes, the world key is reset and the core stops
    fn disable(api: &Api) {
        let Some(runtime) = StratumRuntime::get() else { return };
        runtime.latch().raise();
        Self::remove_hooks(api, runtime);
        runtime.stop();
    }
}
