// The game functions stratum hooks. Each detour calls the original first and only reads or draws after it, except
// the focus loss, which skips the original while the pause screen is not wanted, and the easy place hooks, which read
// before the original and may hide a packet or skip the game's build or press
pub mod build_action;
pub mod build_block;
pub mod client_update;
pub mod container_close;
pub mod container_open;
pub mod db_storage_ctor;
pub mod db_storage_shutdown;
pub mod focus_lost;
pub mod hook_body;
pub mod hud_tick;
pub mod render_level;
pub mod screen_gone;
pub mod stack_response;

pub use hook_body::HookBody;
