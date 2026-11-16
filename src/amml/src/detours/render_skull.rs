use amml_game::StopLatch;
use amml_sdk::DetourGuard;

use crate::detours::HookBody;
use crate::ghost::SkullSlot;
use crate::services::StratumRuntime;
use crate::types::SkullArgs;

// void SkullBlockRenderer::renderSkull(...): the skull draw every skull path shares. While the drawer draws a skull
// ghost, the face, the turn and the name come from the skull slot; the original then draws with them, so this detour
// changes arguments before the original runs
pub static GUARD: DetourGuard = DetourGuard::new("render_skull");

// the signature of the hooked function: this, the context, the metadata and the position in registers, then the
// face, the turn, the name, the forced material and texture, the breaking amount (the whole stack slot), the
// animation value and the glint flag on the stack
pub type RenderSkullFn = unsafe extern "C" fn(usize, usize, usize, usize, i32, f32, usize, usize, usize, u64, f32, u8);

// the latch and the slot the body uses before load installed the runtime; the latch is never raised and the slot
// never set
static IDLE: StopLatch = StopLatch::new();
static EMPTY: SkullSlot = SkullSlot::new();

// the body of the detour, kept apart from the hook so a test can drive it
pub struct RenderSkull;

impl RenderSkull {
    /// Puts the pose and the name `slot` holds into `args`, then calls `original` with them; with an empty slot, a
    /// raised `latch` or a disabled mod, `original` gets `args` as the game passed them.
    pub fn run(slot: &SkullSlot, args: &mut SkullArgs, original: impl FnOnce(&SkullArgs), latch: &StopLatch) {
        Self::guarded(args, latch, |args| Self::swap(slot, args), original);
    }

    /// Runs `swap` on a copy of `args` unless `latch` is raised or the mod is disabled, keeps the copy when `swap`
    /// returns, then calls `original` once. A panic in `swap` raises `latch` and is logged, and `original` still runs
    /// with `args` as the game passed them: the game expects its skull drawn, and a half swap is never forwarded.
    pub fn guarded(
        args: &mut SkullArgs,
        latch: &StopLatch,
        swap: impl FnOnce(&mut SkullArgs),
        original: impl FnOnce(&SkullArgs),
    ) {
        HookBody::on_copy(args, latch, swap, original);
    }

    fn swap(slot: &SkullSlot, args: &mut SkullArgs) {
        if let Some((pose, name)) = slot.get() {
            args.face = i32::from(pose.face);
            args.degrees = pose.degrees;
            args.name = name as usize;
        }
    }
}

#[allow(unsafe_code, clippy::too_many_arguments)]
pub extern "C" fn detour(
    this: usize,
    ctx: usize,
    metadata: usize,
    position: usize,
    face: i32,
    degrees: f32,
    name: usize,
    material: usize,
    texture: usize,
    breaking: u64,
    animation: f32,
    glint: u8,
) {
    let original: RenderSkullFn = GUARD.original_fn();
    let runtime = StratumRuntime::get();
    let mut args =
        SkullArgs { this, ctx, metadata, position, face, degrees, name, material, texture, breaking, animation, glint };
    RenderSkull::run(
        runtime.map_or(&EMPTY, |runtime| runtime.skull_slot()),
        &mut args,
        |a| {
            // SAFETY: `original` is the next function in the chain, with the hooked signature; every argument is the
            // game's, except the face, the turn and the name of a skull ghost, where the name is the live block
            // type's own full name, as the game's skull renderer passes it
            unsafe {
                original(
                    a.this,
                    a.ctx,
                    a.metadata,
                    a.position,
                    a.face,
                    a.degrees,
                    a.name,
                    a.material,
                    a.texture,
                    a.breaking,
                    a.animation,
                    a.glint,
                )
            }
        },
        runtime.map_or(&IDLE, |runtime| runtime.latch()),
    );
}
