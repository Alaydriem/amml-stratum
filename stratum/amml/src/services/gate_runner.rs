use std::ffi::c_void;
use std::ptr::NonNull;

use amml_game::{Block, BlockPos, ClientInstance, GameEntries, RawObject, RenderCamera};
use stratum_core::structs::ghost::GhostPalette;
use stratum_core::structs::structure::{BlockState, BlockStates, StateValue};

use crate::game::{
    BlockRegistry, GameBlockTessellator, NbtBlock, OwnedMesh, ScreenContext, Tessellator, TexturePtrLayout,
    TextureVariant,
};
use crate::types::{GhostCalls, PlayerSpot};

/// The in-game gate of the ghost render path, on the render thread: on the first frame in a world it tessellates
/// one stone block 3 blocks above the player's feet, keeps the mesh and draws it each frame, and looks up one
/// palette entry by its NBT bytes. Each step logs one line. A new world builds again; the mesh of the world that
/// was left is destroyed.
pub struct GateRunner {
    calls: GhostCalls,
    registry: BlockRegistry,
    blocks: NbtBlock,
    entry_nbt: Vec<u8>,
    region: usize,
    origin: BlockPos,
    eye: [f32; 3],
    built: bool,
    mesh: Option<OwnedMesh>,
    frames: u64,
    // the frames line waits while another gate step still runs, so the in-game run does not close the game first
    hold: bool,
    frames_logged: bool,
}

// SAFETY: the raw game pointers and the mesh are used on the render thread only, under the runtime's lock; another
// thread only moves the runner, it never calls through it
#[allow(unsafe_code)]
unsafe impl Send for GateRunner {}

impl GateRunner {
    /// Blocks between the player's feet and the gate block.
    pub const HEIGHT: i32 = 3;
    /// The drawn frames after which the gate logs that it is still running.
    pub const FRAMES_TO_LOG: u64 = 600;
    /// The eye height of a player over its feet: `StateVectorComponent::mPos` of a player is the eye position.
    pub const EYE_HEIGHT: f32 = PlayerSpot::EYE_HEIGHT;
    /// How far under a whole block a feet level still counts as that block: the game keeps a player who stands on
    /// y 72 at eye 73.6198, a little under 72 + EYE_HEIGHT. No block top is this close under a whole block.
    pub const FEET_TOLERANCE: f32 = PlayerSpot::FEET_TOLERANCE;
    /// The vertices of one full block, six quads.
    const BLOCK_VERTICES: i32 = 24;
    const STONE: &'static str = "minecraft:stone";
    const ENTRY_NAME: &'static str = "minecraft:oak_stairs";

    pub fn new(calls: &GhostCalls) -> Self {
        Self {
            calls: *calls,
            registry: BlockRegistry::new(calls),
            blocks: NbtBlock::new(BlockRegistry::new(calls), calls),
            entry_nbt: Self::entry_nbt(),
            region: 0,
            origin: BlockPos { x: 0, y: 0, z: 0 },
            eye: [0.0; 3],
            built: false,
            mesh: None,
            frames: 0,
            hold: false,
            frames_logged: false,
        }
    }

    /// The palette entry the gate looks up: `minecraft:oak_stairs` facing direction 2 (the stairs state
    /// `weirdo_direction`, as the sample structures store it) and not upside down, in the shape of a `.mcstructure`
    /// palette entry (`{ name, states, version }`), written by the ghost palette.
    pub fn entry_nbt() -> Vec<u8> {
        let mut palette = GhostPalette::new();
        palette.entries.push(BlockState::new(
            Self::ENTRY_NAME.to_string(),
            BlockStates::new(vec![
                ("upside_down_bit".to_string(), StateValue::Bool(false)),
                ("weirdo_direction".to_string(), StateValue::Int(2)),
            ]),
        ));
        let mut out = Vec::new();
        if palette.entry_nbt(0, &mut out).is_err() {
            out.clear();
        }
        out
    }

    /// Holds back the `gate frames=` line while `hold` is true; the line is logged on the first drawn frame after
    /// both `FRAMES_TO_LOG` frames and the release.
    pub fn hold_frames_line(&mut self, hold: bool) {
        self.hold = hold;
    }

    /// The block 3 above the feet of a player whose eye is at `eye`.
    pub fn gate_block(eye: [f32; 3]) -> BlockPos {
        let [x, y, z] = PlayerSpot::feet_of(eye);
        BlockPos { x, y: y + Self::HEIGHT, z }
    }

    /// After `LevelRenderer::renderLevel`: `renderer` is its `this`, `screen` its `ScreenContext&` and `frame` its
    /// `FrameRenderObject const&`; `client` is the client the tick saw last. `log` gets each gate line.
    ///
    /// # Safety
    /// The pointers must be the live arguments of the running `renderLevel` call, and `client` null or the live
    /// client of the game.
    #[allow(unsafe_code)]
    pub unsafe fn after_render_level(
        &mut self,
        entries: &GameEntries,
        client: Option<NonNull<c_void>>,
        renderer: *mut c_void,
        screen: *mut c_void,
        frame: *const c_void,
        log: &mut impl FnMut(&str),
    ) {
        // SAFETY: the client the update detour of this frame saw, live while the game runs
        let client = client.and_then(|client| unsafe { ClientInstance::from_raw(client.as_ptr(), entries) });
        let world = client.as_ref().and_then(|client| Some((client.local_player()?, client.region()?)));
        let Some((player, region)) = world else {
            self.leave();
            return;
        };
        if region.as_ptr() as usize != self.region {
            self.leave();
            self.region = region.as_ptr() as usize;
        }
        // SAFETY: the ScreenContext of the running render call
        let Some(screen) = (unsafe { ScreenContext::from_raw(screen, &self.calls) }) else { return };
        if !self.built {
            let Some(pose) = player.pose() else { return };
            self.eye = pose.pos;
            self.origin = Self::gate_block(pose.pos);
            self.built = self.build(&screen, region.as_ptr(), log);
            if !self.built {
                return;
            }
            self.lookup(entries, log);
        }
        if let Some(client) = client.as_ref()
            && self.draw(entries, client, renderer, &screen, frame)
        {
            self.frames += 1;
            if self.frames >= Self::FRAMES_TO_LOG && !self.hold && !self.frames_logged {
                self.frames_logged = true;
                log(&format!("gate frames={} drawn", Self::FRAMES_TO_LOG));
            }
        }
    }

    // the world was left: its mesh is destroyed and the next world builds again
    fn leave(&mut self) {
        self.mesh = None;
        self.region = 0;
        self.built = false;
        self.frames = 0;
        self.frames_logged = false;
    }

    // tessellates the stone block at the gate position into a mesh relative to it; false when the game's
    // tessellator is in use this frame, so the build runs again on the next one
    #[allow(unsafe_code)]
    fn build(&mut self, screen: &ScreenContext, region: *mut c_void, log: &mut impl FnMut(&str)) -> bool {
        let Some(tessellator) = screen.tessellator() else { return false };
        // SAFETY: the tessellator of the running render call
        let Some(mut tessellator) = (unsafe { Tessellator::from_raw(tessellator, &self.calls) }) else { return false };
        if tessellator.is_tessellating() {
            return false;
        }
        let stone = self.registry.by_name(Self::STONE);
        if stone.is_null() {
            log("gate mesh valid=false verts=0 (no minecraft:stone block)");
            return true;
        }
        // SAFETY: the client's live region; the game heap through the calls
        let Some(mut blocks) = (unsafe { GameBlockTessellator::new(region, &self.calls) }) else {
            log("gate mesh valid=false verts=0 (no block tessellator)");
            return true;
        };
        let origin = self.origin;
        let shift = [origin.x as f32, origin.y as f32, origin.z as f32];
        let build = tessellator.build_quads(Self::BLOCK_VERTICES, "stratum_gate", |tessellator| {
            // SAFETY: the stone block of this world and the client's live region
            let filled = unsafe { blocks.tessellate(tessellator, stone, origin) };
            tessellator.shift_positions(shift);
            filled
        });
        // SAFETY: the region is still live, so the destructor reads no freed BlockSource
        unsafe { blocks.set_region(region) };
        drop(blocks);
        let Some(build) = build else { return false };
        let valid = build.mesh.as_ref().is_some_and(OwnedMesh::is_valid);
        log(&format!(
            "gate mesh valid={valid} verts={} filled={} at {} {} {} eye {} {} {}",
            build.vertices, build.filled, origin.x, origin.y, origin.z, self.eye[0], self.eye[1], self.eye[2]
        ));
        self.mesh = if valid { build.mesh } else { None };
        true
    }

    #[allow(unsafe_code)]
    fn lookup(&self, entries: &GameEntries, log: &mut impl FnMut(&str)) {
        let found = self.blocks.lookup(&self.entry_nbt);
        // SAFETY: a block the registry gave for this world, or null
        let block = unsafe { Block::from_raw(found.block().cast_mut(), entries) };
        let name = block.as_ref().and_then(Block::type_name).and_then(|name| name.to_str().ok()).unwrap_or("none");
        log(&format!("gate nbt kind={} name={name}", found.kind()));
    }

    // draws the mesh at the gate position, relative to the camera; true when it was drawn
    #[allow(unsafe_code)]
    fn draw(
        &self,
        entries: &GameEntries,
        client: &ClientInstance,
        renderer: *mut c_void,
        screen: &ScreenContext,
        frame: *const c_void,
    ) -> bool {
        let Some(mesh) = self.mesh.as_ref() else { return false };
        if renderer.is_null() || frame.is_null() {
            return false;
        }
        // SAFETY: the live FrameRenderObject of the call; `mViews` is a std::vector<ViewRenderObject>
        let views = unsafe { RawObject::read::<[usize; 2]>(frame, self.calls.frame_views) };
        let Some([first, last]) = views else { return false };
        if first == 0 || first == last {
            return false;
        }
        // SAFETY: the first ViewRenderObject of the frame
        let Some(camera) = (unsafe { RenderCamera::new(entries).view_position(first as *const c_void) }) else {
            return false;
        };
        let Some(material) = self.blend_material(client) else { return false };
        // SAFETY: the live LevelRenderer of the call holds its atlas TexturePtr at the entry's offset
        let atlas =
            unsafe { &*renderer.cast::<u8>().add(self.calls.atlas_texture as usize).cast::<TexturePtrLayout>() };
        // SAFETY: the renderer's atlas holds live uses; the copy takes its own for the frame
        let mut texture = unsafe { atlas.clone_ref() };
        let variant = TextureVariant::new(TexturePtrLayout::VARIANT_INDEX, texture);
        let offset =
            [self.origin.x as f32 - camera[0], self.origin.y as f32 - camera[1], self.origin.z as f32 - camera[2]];
        screen.set_ghost_constants();
        let pushed = screen.push_world_translation(offset);
        if pushed.is_some() {
            // SAFETY: the item-in-hand renderer's material and a texture copy that holds its own uses
            unsafe { mesh.render(screen, material, &variant) };
        }
        drop(pushed);
        // SAFETY: the copy holds the two uses `clone_ref` took; the variant was a byte copy and is not released
        unsafe { texture.release() };
        true
    }

    // the blend-block material of the client's item-in-hand renderer, as a `mce::MaterialPtr const*`
    #[allow(unsafe_code)]
    fn blend_material(&self, client: &ClientInstance) -> Option<*const c_void> {
        // SAFETY: a live ClientInstance; the entry is the slot of `getItemInHandRenderer`
        let function = unsafe { RawObject::slot(client.as_ptr(), self.calls.item_in_hand_renderer_slot) }?;
        // SAFETY: the slot holds `ItemInHandRenderer* ClientInstance::getItemInHandRenderer()`
        let renderer = unsafe { RawObject::call_pointer(function, client.as_ptr()) }?;
        Some(renderer.cast::<u8>().wrapping_add(self.calls.blend_material as usize).cast::<c_void>().cast_const())
    }
}
