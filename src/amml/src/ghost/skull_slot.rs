use std::ffi::c_void;
use std::sync::atomic::{AtomicPtr, AtomicU8, AtomicU32, Ordering};

use stratum_core::structs::ghost::SkullPose;

/// The pose of the one skull ghost being drawn: the drawer sets it right before the dispatcher draws a skull cell and
/// clears it right after, and the `renderSkull` detour reads it inside that call. It holds at most one pose. Render
/// thread only: the set, the read in the detour and the clear run in that order on one thread, so relaxed loads and
/// stores are enough.
pub struct SkullSlot {
    // `HashedString const*` of the cell's block type full name; null while the slot is empty
    name: AtomicPtr<c_void>,
    face: AtomicU8,
    // the turn in degrees, as `f32` bits
    degrees: AtomicU32,
}

impl SkullSlot {
    pub const fn new() -> Self {
        Self { name: AtomicPtr::new(std::ptr::null_mut()), face: AtomicU8::new(0), degrees: AtomicU32::new(0) }
    }

    /// Holds `pose` with `name`, the cell's block type full name. A null `name` leaves the slot empty.
    pub fn set(&self, pose: SkullPose, name: *const c_void) {
        self.face.store(pose.face, Ordering::Relaxed);
        self.degrees.store(pose.degrees.to_bits(), Ordering::Relaxed);
        self.name.store(name.cast_mut(), Ordering::Relaxed);
    }

    pub fn clear(&self) {
        self.name.store(std::ptr::null_mut(), Ordering::Relaxed);
    }

    /// The pose and the name pointer, or `None` while the slot is empty.
    pub fn get(&self) -> Option<(SkullPose, *const c_void)> {
        let name = self.name.load(Ordering::Relaxed);
        if name.is_null() {
            return None;
        }
        let degrees = f32::from_bits(self.degrees.load(Ordering::Relaxed));
        Some((SkullPose::new(self.face.load(Ordering::Relaxed), degrees), name.cast_const()))
    }
}

impl Default for SkullSlot {
    fn default() -> Self {
        Self::new()
    }
}
