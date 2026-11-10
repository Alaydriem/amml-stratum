use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError, TryLockError};

use crate::ghost::GhostRenderer;
use crate::types::GhostCalls;

// The ghost renderer of the runtime, for the render thread. Enable installs it, or asks the next frame to reset the
// one of the last enable; a lock a render body poisoned with a panic is cleared at the next enable, and the reset then
// frees the meshes that panic left. While the lock is poisoned no frame runs, and that is logged once
pub struct RendererSlot {
    renderer: Mutex<Option<GhostRenderer>>,
    reset: AtomicBool,
    poison_logged: AtomicBool,
}

impl RendererSlot {
    pub fn new() -> Self {
        Self { renderer: Mutex::new(None), reset: AtomicBool::new(false), poison_logged: AtomicBool::new(false) }
    }

    // enable: a renderer for `calls`, or the one of the last enable with a reset on its next frame
    pub fn install(&self, calls: &GhostCalls) {
        if self.renderer.is_poisoned() {
            curia::warn!(
                "the ghost renderer lock was poisoned by a render panic; it is cleared and the ghost resets",
                {}
            );
            self.renderer.clear_poison();
        }
        let mut renderer = self.renderer.lock().unwrap_or_else(PoisonError::into_inner);
        if renderer.is_none() {
            *renderer = Some(GhostRenderer::new(calls));
        } else {
            self.reset.store(true, Ordering::SeqCst);
        }
        self.poison_logged.store(false, Ordering::SeqCst);
    }

    // render thread: runs `body` with the renderer, after the reset an enable asked for; false when no frame ran,
    // because there is no renderer, another frame holds it, or a panic poisoned it
    pub fn frame(&self, body: impl FnOnce(&mut GhostRenderer)) -> bool {
        let mut renderer = match self.renderer.try_lock() {
            Ok(renderer) => renderer,
            Err(TryLockError::WouldBlock) => return false,
            Err(TryLockError::Poisoned(_)) => {
                if !self.poison_logged.swap(true, Ordering::SeqCst) {
                    curia::error!(
                        "the ghost renderer lock is poisoned; the ghost does not draw until the next enable",
                        {}
                    );
                }
                return false;
            }
        };
        let Some(renderer) = renderer.as_mut() else { return false };
        if self.reset.swap(false, Ordering::SeqCst) {
            renderer.reset();
        }
        body(renderer);
        true
    }

    // game thread: the block of palette id `id`, as the renderer resolved it on the render thread; null with no
    // renderer or a poisoned lock. The render thread holds the lock only for its own frame body, which never waits on
    // the game thread, so the wait is short
    pub fn palette_block(&self, id: u16) -> *const std::ffi::c_void {
        match self.renderer.lock() {
            Ok(renderer) => renderer.as_ref().map_or(std::ptr::null(), |renderer| renderer.palette_block(id)),
            Err(_) => std::ptr::null(),
        }
    }
}

impl Default for RendererSlot {
    fn default() -> Self {
        Self::new()
    }
}
