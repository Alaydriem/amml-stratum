use std::ffi::c_void;

use amml_game::{Aabb, HitResultLayout, RawObject};

use crate::types::{PlayerCalls, UseCalls};

// `Vec3 Actor::getAttachPos(ActorLocation, float) const`: this, the hidden return pointer, the location, the alpha
// (xmm3)
type AttachPosFn = unsafe extern "C" fn(*mut c_void, *mut [f32; 3], i32, f32) -> *mut [f32; 3];

// `Vec3 Actor::getViewVector(float) const`: this, the hidden return pointer, the alpha (xmm2)
type ViewVectorFn = unsafe extern "C" fn(*mut c_void, *mut [f32; 3], f32) -> *mut [f32; 3];

// `GameType Player::getPlayerGameType() const`: the enum in eax
type GameTypeFn = unsafe extern "C" fn(*mut c_void) -> i32;

/// What the local player sees and is: the ray start and direction of its use, the box it fills, its game type, and
/// the level's hit result, which the game's own use reads.
pub struct PlayerView {
    player: *mut c_void,
    calls: UseCalls,
    players: PlayerCalls,
}

impl PlayerView {
    /// `SharedTypes::Legacy::ActorLocation::Head`: the game's own pick takes its ray start from
    /// `getAttachPos(Head, 0)`, as LeviLamina's `Actor::getHeadPos` does.
    pub const HEAD_LOCATION: i32 = 3;

    /// The view of `player`; None for a null player.
    ///
    /// # Safety
    /// `player` must be null or a live local `Player` of the build the calls were resolved for, live while the view
    /// is used.
    #[allow(unsafe_code)]
    pub unsafe fn of_player(player: *mut c_void, calls: &UseCalls, players: &PlayerCalls) -> Option<Self> {
        (!player.is_null()).then_some(Self { player, calls: *calls, players: *players })
    }

    /// The ray start of the use: `getAttachPos(Head, 0)`.
    pub fn eye(&self) -> [f32; 3] {
        if self.calls.get_attach_pos == 0 {
            return [0.0; 3];
        }
        // SAFETY: the entry is the address of getAttachPos
        #[allow(unsafe_code)]
        let call: AttachPosFn = unsafe { std::mem::transmute(self.calls.get_attach_pos) };
        let mut out = [0.0f32; 3];
        // SAFETY: the live player and a Vec3 buffer the callee fills
        #[allow(unsafe_code)]
        unsafe {
            call(self.player, &raw mut out, Self::HEAD_LOCATION, 0.0)
        };
        out
    }

    /// `getViewVector(alpha)`.
    pub fn view_vector(&self, alpha: f32) -> [f32; 3] {
        if self.calls.get_view_vector == 0 {
            return [0.0; 3];
        }
        // SAFETY: the entry is the address of getViewVector
        #[allow(unsafe_code)]
        let call: ViewVectorFn = unsafe { std::mem::transmute(self.calls.get_view_vector) };
        let mut out = [0.0f32; 3];
        // SAFETY: the live player and a Vec3 buffer the callee fills
        #[allow(unsafe_code)]
        unsafe {
            call(self.player, &raw mut out, alpha)
        };
        out
    }

    /// The player's box, as LeviLamina's `Actor::getAABB` reads it: the AABB at the start of its
    /// `AABBShapeComponent`; None for a null component.
    pub fn body(&self) -> Option<Aabb> {
        // SAFETY: the live player holds the component pointer at the entry's offset
        #[allow(unsafe_code)]
        let component = unsafe { RawObject::pointer(self.player, self.calls.aabb_shape_component) }?;
        // SAFETY: a live AABBShapeComponent, whose AABB comes first
        #[allow(unsafe_code)]
        unsafe {
            RawObject::read::<Aabb>(component, 0)
        }
    }

    /// `getPlayerGameType()`; None when the entry is 0.
    pub fn game_type(&self) -> Option<i32> {
        if self.players.get_player_game_type == 0 {
            return None;
        }
        // SAFETY: the entry is the address of getPlayerGameType
        #[allow(unsafe_code)]
        let call: GameTypeFn = unsafe { std::mem::transmute(self.players.get_player_game_type) };
        // SAFETY: the live player
        #[allow(unsafe_code)]
        Some(unsafe { call(self.player) })
    }

    /// The level's `HitResult` (`getLevel().getHitResult()`), as the mirror of its start; None for a null level or
    /// result.
    pub fn hit_result(&self) -> Option<*mut HitResultLayout> {
        // SAFETY: the live player holds its Level pointer at the entry's offset
        #[allow(unsafe_code)]
        let level = unsafe { RawObject::pointer(self.player, self.players.actor_level) }?;
        // SAFETY: a live Level, whose vtable holds getHitResult at the entry's slot
        #[allow(unsafe_code)]
        let function = unsafe { RawObject::slot(level, self.calls.level_get_hit_result_slot) }?;
        // SAFETY: the slot holds `HitResult& Level::getHitResult()`, a reference returned as a pointer
        #[allow(unsafe_code)]
        let hit = unsafe { RawObject::call_pointer(function, level) }?;
        Some(hit.cast())
    }
}
