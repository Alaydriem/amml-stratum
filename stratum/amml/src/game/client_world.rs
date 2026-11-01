use std::ffi::c_void;

use amml_game::{ClientInstance, EntryId, GameEntries, GameHeap, RawObject};

use crate::game::{ClientLevel, GameConnectionInfo};
use crate::services::WorldFolder;
use crate::traits::{ClientSource, WorldSource};
use crate::types::{ConnectionInfo, PlayerSpot, StratumEntry};

/// What one tick reads from the game's `ClientInstance*`, for the frame status and the world key. Game thread; no
/// read is thread-checked.
pub struct ClientWorld<'a> {
    client: ClientInstance<'a>,
    entries: &'a GameEntries,
    folder: &'a WorldFolder,
}

impl<'a> ClientWorld<'a> {
    /// `None` for a null pointer. `folder` holds the folder of the local world the storage detour saw.
    ///
    /// # Safety
    /// `client` must be null or the live `ClientInstance` of the game build `entries` was resolved for, live while
    /// the value is used.
    #[allow(unsafe_code)]
    pub unsafe fn from_raw(client: *mut c_void, entries: &'a GameEntries, folder: &'a WorldFolder) -> Option<Self> {
        // SAFETY: forwarded to the caller
        let client = unsafe { ClientInstance::from_raw(client, entries) }?;
        Some(Self { client, entries, folder })
    }

    // the function in the vtable slot `id` names, on `object`
    #[allow(unsafe_code)]
    fn function(&self, object: *mut c_void, id: EntryId) -> Option<*const c_void> {
        let slot = self.entries.get(id)?;
        // SAFETY: the callers pass a live ClientInstance or IOptionRegistry, whose first qword is its vtable, and
        // `id` is a slot entry of that class
        unsafe { RawObject::slot(object, slot) }
    }

    #[allow(unsafe_code)]
    fn client_level(&self) -> Option<ClientLevel<'a>> {
        let function = self.function(self.client.as_ptr(), StratumEntry::CLIENT_INSTANCE_GET_LEVEL)?;
        // SAFETY: the slot holds `ClientLevel* ClientInstance::getLevel()`
        let level = unsafe { RawObject::call_pointer(function, self.client.as_ptr()) }?;
        // SAFETY: the game returns its live ClientLevel, or null out of a world
        unsafe { ClientLevel::from_raw(level, self.entries) }
    }
}

impl WorldSource for ClientWorld<'_> {
    fn level(&self) -> Option<usize> {
        self.client_level().map(|level| level.address())
    }

    #[allow(unsafe_code)]
    fn in_realm(&self) -> bool {
        self.function(self.client.as_ptr(), StratumEntry::CLIENT_INSTANCE_IS_IN_REALM).is_some_and(|function| {
            // SAFETY: the slot holds `bool ClientInstance::isInRealm()`
            unsafe { RawObject::call_bool(function, self.client.as_ptr()) }
        })
    }

    #[allow(unsafe_code)]
    fn connection(&self) -> Option<ConnectionInfo> {
        let heap = GameHeap::msvc()?;
        // SAFETY: the wrapper holds the live ClientInstance
        let info = unsafe { GameConnectionInfo::from_raw(self.client.as_ptr(), self.entries) }?;
        info.read(&heap)
    }

    // the storage folder names the world only while both of its sources are on
    fn local_folder(&self) -> Option<String> {
        self.entries.get(StratumEntry::CLIENT_INSTANCE_GET_GAME_CONNECTION_INFO)?;
        self.entries.get(StratumEntry::DB_STORAGE_CTOR)?;
        self.folder.current()
    }

    fn level_name(&self) -> Option<String> {
        self.client_level()?.level_name()
    }

    fn seed(&self) -> Option<u64> {
        self.client_level()?.seed()
    }
}

impl ClientSource for ClientWorld<'_> {
    fn is_world_active(&self) -> bool {
        self.client.is_world_active()
    }

    // a player whose position or dimension cannot be read counts as no player
    fn player(&self) -> Option<PlayerSpot> {
        let player = self.client.local_player()?;
        let pose = player.pose()?;
        let dimension = player.dimension()?.id()?;
        Some(PlayerSpot { dimension, feet: PlayerSpot::feet_of(pose.pos) })
    }

    fn menu_open(&self) -> bool {
        self.client.is_showing_menu()
    }

    #[allow(unsafe_code)]
    fn view_distance_chunks(&self) -> i32 {
        let Some(get_options) = self.function(self.client.as_ptr(), EntryId::CLIENT_INSTANCE_GET_OPTIONS) else {
            return 0;
        };
        // SAFETY: the slot holds `IOptionRegistry& ClientInstance::getOptions()`, a reference returned as a pointer
        let Some(options) = (unsafe { RawObject::call_pointer(get_options, self.client.as_ptr()) }) else { return 0 };
        let Some(function) = self.function(options, StratumEntry::I_OPTION_REGISTRY_GET_VIEW_DISTANCE_CHUNKS) else {
            return 0;
        };
        // SAFETY: the slot holds `int IOptionRegistry::getViewDistanceChunks() const`
        let call: unsafe extern "C" fn(*mut c_void) -> i32 = unsafe { std::mem::transmute(function) };
        // SAFETY: `options` is the live option registry the game returned
        unsafe { call(options) }
    }
}
