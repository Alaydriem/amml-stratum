use std::ffi::c_void;

use amml_game::{BlockPos, RawObject};
use stratum_core::structs::ghost::{GhostBlockEntity, GhostBlockEntityTable, SkullPose};

use crate::game::{BlockCalls, GameBlockActor};
use crate::ghost::{BlockActorKey, BlockActorPool, BlockResolver, SkullSlot};
use crate::types::{BlockActorCalls, GhostCalls};

// void BlockActorRenderDispatcher::render(BaseActorRenderContext&, BlockSource&, IVanillaRenderBlockActorComponent&,
// Block const*, Vec3 const& renderPos, BlockPos const& worldPos, bool renderAlphaLayer, MaterialPtr const* forcedMat,
// ClientTexture const* forceTex, int breakingAmount, std::optional<RenderMetadata>*): the form that takes a render
// position, `this` first
type RenderFn = unsafe extern "C" fn(
    *mut c_void,
    *mut c_void,
    *mut c_void,
    *mut c_void,
    *const c_void,
    *const [f32; 3],
    *const BlockPos,
    u8,
    *const c_void,
    *const c_void,
    i32,
    *mut c_void,
);

/// Draws the block-entity cells of the ghost with the game's block actor renderer, opaque and untinted, in the
/// game's own block-entity pass. One block actor serves every cell of its key; the actors live in a bounded pool
/// that the renderer clears when it leaves a world or the palette changes. The material and metadata buffers the
/// dispatcher reads are owned here and reused, so a draw allocates nothing. Render thread only.
pub struct BlockActorDrawer {
    game: BlockCalls,
    pool: BlockActorPool<GameBlockActor>,
    // an empty `mce::MaterialPtr`: no forced material
    material: [u64; 2],
    // a disengaged `std::optional<RenderMetadata>`; the engaged flag is the byte at 0x38
    metadata: [u64; 8],
}

impl BlockActorDrawer {
    pub fn new(calls: &GhostCalls) -> Self {
        Self { game: BlockCalls::new(calls), pool: BlockActorPool::new(), material: [0; 2], metadata: [0; 8] }
    }

    /// Drops every block actor and forgets every failed make. A palette change or a world exit calls it.
    pub fn clear(&mut self) {
        self.pool.clear();
    }

    /// The block actors the pool holds.
    pub fn actors(&self) -> usize {
        self.pool.len()
    }

    /// Renders each cell of `cells`, a key, the cell's world position and its skull pose, through the dispatcher of
    /// `ctx`, at its position relative to the context's camera target. A cell's actor comes from the pool, or is made
    /// from the key's palette block and its NBT in `table` while the frame's make budget lasts; the actor of a large
    /// chest lead is made a large chest once, when it is made, and only when the game's `isType(Chest)` says it is a
    /// chest; any other actor draws as it is, and with the large chests off a lead draws as a single chest. A cell with
    /// a skull pose holds that pose and its block type's full name in `skull` for the dispatcher call only, so the
    /// skull hook draws the ghost's skull; the slot is empty before and after. A cell with no block, no actor or no
    /// render component, or a skull cell with the skulls off, is not drawn. Returns how many cells were rendered.
    ///
    /// # Safety
    /// `calls` must hold the entries of the running build; `blocks` must be resolved for the joined world; `ctx` must
    /// be null or the live `BaseActorRenderContext` of the running block-entity pass; `region` must be the live client
    /// region and `level` null or the live client level. Call it on the render thread.
    #[allow(unsafe_code, clippy::too_many_arguments)]
    pub unsafe fn draw(
        &mut self,
        calls: &BlockActorCalls,
        blocks: &BlockResolver,
        ctx: *mut c_void,
        region: *mut c_void,
        level: *mut c_void,
        table: Option<&GhostBlockEntityTable>,
        skull: &SkullSlot,
        cells: impl Iterator<Item = (BlockActorKey, BlockPos, Option<SkullPose>)>,
    ) -> u32 {
        if ctx.is_null() || region.is_null() {
            return 0;
        }
        // SAFETY: the caller's live render context, which holds the dispatcher pointer at the entry's offset
        let Some(dispatcher) = (unsafe { RawObject::pointer(ctx, calls.context_dispatcher) }) else { return 0 };
        // SAFETY: as above, for the context's Impl pointer
        let Some(context_impl) = (unsafe { RawObject::pointer(ctx, calls.context_impl) }) else { return 0 };
        // SAFETY: the context's live Impl, which holds the camera target Vec3 at the entry's offset
        let Some(camera) = (unsafe { RawObject::read::<[f32; 3]>(context_impl, calls.impl_camera_target) }) else {
            return 0;
        };
        // SAFETY: the entry is the address of the dispatcher's render with the signature of `RenderFn`
        let render: RenderFn = unsafe { std::mem::transmute::<usize, RenderFn>(calls.dispatcher_render) };
        self.pool.begin_frame();
        let game = self.game;
        let Self { pool, material, metadata, .. } = self;
        let mut drawn = 0;
        for (key, world, pose) in cells {
            if pose.is_some() && calls.skull.is_none() {
                continue;
            }
            let block = blocks.block(key.palette);
            if block.is_null() || block == blocks.air() {
                continue;
            }
            let nbt = if key.entry == GhostBlockEntity::NONE { None } else { table.and_then(|t| t.entry(key.entry)) };
            // SAFETY: a block of the joined world
            let block_type = unsafe { game.block_type(block) };
            let made = if level.is_null() && nbt.is_some_and(|bytes| !bytes.is_empty()) {
                // a load from NBT needs the level: the cell waits for a pass that has one, and the key is not
                // remembered as failed
                pool.get(key)
            } else {
                pool.get_or_make(key, || {
                    // SAFETY: forwarded to the caller: the entries, a live block type, its NBT from stratum's parser
                    // and the live level
                    let mut actor = unsafe { GameBlockActor::load(calls, block_type, world, nbt, level) }?;
                    if let Some(pair) = key.chest
                        && let Some(chest) = calls.chest
                        && actor.is_chest(&chest)
                    {
                        // SAFETY: the caller guarantees the entries; the game's own type check says the actor is a
                        // ChestBlockActor
                        unsafe { actor.set_large_chest(&chest, pair) };
                    }
                    Some(actor)
                })
            };
            let Some(actor) = made else { continue };
            let component = actor.render_component(calls);
            if component.is_null() {
                continue;
            }
            let render_pos = [world.x as f32 - camera[0], world.y as f32 - camera[1], world.z as f32 - camera[2]];
            *material = [0; 2];
            *metadata = [0; 8];
            if let Some(pose) = pose
                && let Some(skulls) = calls.skull
                && !block_type.is_null()
            {
                skull.set(pose, block_type.wrapping_byte_add(skulls.block_type_full_name as usize));
            }
            // SAFETY: the context's live dispatcher, the caller's live context and region, the actor's live
            // component, a block of the world, and buffers that live for the call
            unsafe {
                render(
                    dispatcher,
                    ctx,
                    region,
                    component,
                    block,
                    &render_pos,
                    &world,
                    0,
                    material.as_ptr().cast(),
                    std::ptr::null(),
                    0,
                    metadata.as_mut_ptr().cast(),
                )
            };
            skull.clear();
            drawn += 1;
        }
        drawn
    }
}
