use std::sync::Arc;

use amml_api::amml_abi::{
    AppFocusLost, BlockChanged, BlockEntityPass, BuildBlock, ClientTick, DimensionChanging, InventoryRequestDone,
    MouseInput, ServerJoinStarting, UsePressed, WindowFocus, WorldJoined, WorldLeft, WorldRendered,
};
use amml_api::{
    ApiError, Cancel, Ctx, HookOrder, InventoryRequestDoneExt, Listener, ServerJoinStartingExt, WindowFocusExt,
};

use crate::services::StratumRuntime;

// the loader listeners of stratum; each only hands its event to the runtime. Dropping them removes them
pub struct StratumListeners {
    listeners: Vec<Listener>,
}

impl StratumListeners {
    pub const COUNT: usize = 14;

    pub fn start(ctx: &Ctx, runtime: &Arc<StratumRuntime>) -> Result<Self, ApiError> {
        let events = ctx.events();
        let mut listeners = Vec::with_capacity(Self::COUNT);
        let read = HookOrder::READ_AFTER_WRITERS;
        let to = Arc::clone(runtime);
        listeners.push(events.listen::<ClientTick>(read, move |_, at| to.tick(at))?);
        let to = Arc::clone(runtime);
        listeners.push(events.listen::<WorldJoined>(HookOrder::NORMAL, move |event, _| to.world_joined(event.epoch))?);
        let to = Arc::clone(runtime);
        listeners.push(events.listen::<ServerJoinStarting>(HookOrder::NORMAL, move |event, _| {
            to.server_join_starting(event.url(), event.name(), event.host(), event.port());
        })?);
        let to = Arc::clone(runtime);
        listeners.push(events.listen::<WorldLeft>(HookOrder::NORMAL, move |_, _| to.world_left())?);
        let to = Arc::clone(runtime);
        listeners.push(events.listen::<DimensionChanging>(HookOrder::NORMAL, move |_, _| to.dimension_changing())?);
        let to = Arc::clone(runtime);
        listeners.push(events.listen::<WorldRendered>(read, move |event, at| to.world_rendered(event, at))?);
        let to = Arc::clone(runtime);
        listeners.push(events.listen::<BlockEntityPass>(read, move |event, at| to.block_entity_pass(event, at))?);
        let to = Arc::clone(runtime);
        listeners.push(events.listen::<BlockChanged>(HookOrder::NORMAL, move |event, _| to.block_changed(event))?);
        let to = Arc::clone(runtime);
        listeners.push(events.listen::<UsePressed>(HookOrder::WRITE, move |event, at| {
            if to.use_pressed(at) {
                event.cancel();
            }
        })?);
        let to = Arc::clone(runtime);
        listeners.push(events.listen::<BuildBlock>(HookOrder::WRITE, move |event, _| {
            if to.blocks_game_build(event) {
                event.cancel();
            }
        })?);
        let to = Arc::clone(runtime);
        listeners.push(events.listen::<InventoryRequestDone>(HookOrder::NORMAL, move |event, _| {
            to.request_done(event.id(), event.outcome());
        })?);
        let to = Arc::clone(runtime);
        listeners.push(events.listen::<MouseInput>(read, move |event, _| to.mouse(event))?);
        let to = Arc::clone(runtime);
        listeners.push(events.listen::<WindowFocus>(HookOrder::NORMAL, move |event, _| to.focus(event.gained()))?);
        let to = Arc::clone(runtime);
        listeners.push(events.listen::<AppFocusLost>(HookOrder::WRITE, move |event, _| {
            if to.pause_screen_unwanted() {
                event.cancel();
            }
        })?);
        Ok(Self { listeners })
    }

    // removes every listener; when this returns no listener call runs
    pub fn stop(self) {
        drop(self.listeners);
    }
}
