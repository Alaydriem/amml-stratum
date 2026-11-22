use std::sync::{Arc, Mutex};

use alaydriem_stratum::ghost::GhostRenderer;
use alaydriem_stratum::services::SharedGhost;
use amml_api::amml_abi::{BlockEntityPass, WorldRendered};
use amml_api::testing::{FakeCanvas, FakeClient, FakeGame};
use amml_api::{BlockEntityCamera, BlockHandle, HookOrder, Listener};
use stratum_core::structs::ghost::{
    GhostBlockEntity, GhostBlockEntityTable, GhostDraw, GhostFrame, GhostJob, GhostPalette, SectionKey, SectionPart,
};
use stratum_core::structs::structure::{BlockState, BlockStates};

use super::StratumRig;

// a ghost renderer driven by a fake game's world render and block entity pass, fed from a `SharedGhost` the test
// publishes frames into
pub struct RenderRig {
    pub game: FakeGame,
    pub ghost: Arc<SharedGhost>,
    pub renderer: Arc<Mutex<GhostRenderer>>,
    pub drawn: Arc<Mutex<Vec<u32>>>,
    _listeners: Vec<Listener>,
}

impl RenderRig {
    pub const CHEST: u64 = 10;
    pub const SKULL: u64 = 11;
    pub const ORIGIN: [i32; 3] = [5, 64, 5];

    pub fn new() -> Self {
        let game = FakeGame::new();
        game.offer::<WorldRendered>();
        game.offer::<BlockEntityPass>();
        for area in StratumRig::areas() {
            game.offer_area(area);
        }
        game.open_gate();
        game.set_epoch(1);
        let mut world = StratumRig::world();
        world.states.insert(("minecraft:chest".to_string(), Vec::new()), BlockHandle::for_test(Self::CHEST));
        world.states.insert(("minecraft:skeleton_skull".to_string(), Vec::new()), BlockHandle::for_test(Self::SKULL));
        game.set_client(FakeClient {
            local_player: true,
            world_active: true,
            world: Some(world),
            ..FakeClient::default()
        });
        game.set_block_entity_kind(BlockHandle::for_test(Self::CHEST), "chest");
        game.set_block_entity_kind(BlockHandle::for_test(Self::SKULL), "skull");
        let ghost = SharedGhost::new_shared();
        let renderer = Arc::new(Mutex::new(GhostRenderer::new(true)));
        let drawn = Arc::new(Mutex::new(Vec::new()));
        let ctx = game.ctx();
        let (frame_ghost, frame_renderer) = (Arc::clone(&ghost), Arc::clone(&renderer));
        let world_rendered = ctx
            .events()
            .listen::<WorldRendered>(HookOrder::READ_AFTER_WRITERS, move |event, at| {
                let (blocks, entities) = (ctx.blocks(), ctx.block_entities());
                frame_renderer.lock().unwrap().frame(event, at.epoch.0, &blocks, &entities, &frame_ghost);
            })
            .unwrap();
        let (pass_renderer, pass_drawn) = (Arc::clone(&renderer), Arc::clone(&drawn));
        let pass = ctx
            .events()
            .listen::<BlockEntityPass>(HookOrder::READ_AFTER_WRITERS, move |event, at| {
                let count = pass_renderer.lock().unwrap().block_entity_pass(event, at.epoch.0, &ctx.block_entities());
                pass_drawn.lock().unwrap().push(count);
            })
            .unwrap();
        Self { game, ghost, renderer, drawn, _listeners: vec![world_rendered, pass] }
    }

    pub fn state(name: &str) -> BlockState {
        BlockState::new(name.to_string(), BlockStates::default())
    }

    // a frame of generation 1 in world epoch 1 with `palette`, one Replace job of one cell of palette id 0 at
    // `ORIGIN` in slot 0, holding `entities`, and the draw of slot 0
    pub fn frame(palette: &[&str], entities: Vec<GhostBlockEntity>) -> GhostFrame {
        let key = SectionKey { x: 0, y: 4, z: 0, layer: None, part: SectionPart::Whole };
        let mut job = GhostJob::new(key);
        job.origin = Self::ORIGIN;
        job.dims = [1, 1, 1];
        job.cells = vec![u16::MAX; 27];
        job.cells[13] = 0;
        job.slot_size = [1, 1, 1];
        job.block_entities = entities;
        let mut ghost_palette = GhostPalette::new();
        ghost_palette.entries = palette.iter().map(|name| Self::state(name)).collect();
        GhostFrame {
            generation: 1,
            world_epoch: 1,
            in_world: true,
            slot_capacity: 4,
            draws: vec![GhostDraw { slot: 0, origin: Self::ORIGIN, offset: [0; 3], scale: 1.0 }],
            jobs: vec![job],
            palette: Some(Arc::new(ghost_palette)),
            block_entities: Some(Arc::new(GhostBlockEntityTable::new(Vec::new()))),
            ..GhostFrame::new()
        }
    }

    // the same build again with no job: the draws only
    pub fn repeat() -> GhostFrame {
        GhostFrame {
            generation: 1,
            world_epoch: 1,
            in_world: true,
            slot_capacity: 4,
            draws: vec![GhostDraw { slot: 0, origin: Self::ORIGIN, offset: [0; 3], scale: 1.0 }],
            ..GhostFrame::new()
        }
    }

    pub fn render(&self, canvas: &FakeCanvas) {
        self.game.raise_world_rendered(canvas);
    }

    pub fn pass(&self, camera: BlockEntityCamera) -> u32 {
        self.game.raise_block_entity_pass(camera);
        self.drawn.lock().unwrap().last().copied().unwrap_or(0)
    }
}
