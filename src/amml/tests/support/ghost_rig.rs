use std::sync::Arc;

use alaydriem_stratum::game::{MeshContextOffset, ScreenContext, TexturePtrLayout};
use alaydriem_stratum::ghost::GhostRenderer;
use alaydriem_stratum::services::SharedGhost;
use alaydriem_stratum::types::{GhostCalls, GhostWorld};
use amml_game::{BlockPos, CameraLayout};
use stratum_core::structs::ghost::{
    GhostDraw, GhostFrame, GhostJob, GhostJobMode, GhostPalette, GhostReport, SectionKey, SectionPart,
};
use stratum_core::structs::structure::{BlockState, BlockStates};

use crate::support::{FakeBuffer, FakeGhost};

/// A ghost renderer on fake game objects: a tessellator, a screen context whose mesh context holds a camera, and
/// the shared ghost the core would publish into. Each frame runs with the camera at the origin.
pub struct GhostRig {
    pub calls: GhostCalls,
    pub ghost: Arc<SharedGhost>,
    pub renderer: GhostRenderer,
    pub tessellator: FakeBuffer,
    screen: FakeBuffer,
    _camera: FakeBuffer,
    atlas: Box<TexturePtrLayout>,
}

impl GhostRig {
    pub const REGION_A: usize = 0xA000;
    pub const REGION_B: usize = 0xB000;
    pub const FEET: BlockPos = BlockPos { x: 0, y: 64, z: 0 };
    /// The slots of the core's table in every rig frame.
    pub const SLOTS: u32 = 8;
    const BLEND: usize = 0xB1E0;

    pub fn new() -> Self {
        let calls = FakeGhost::calls();
        let mut tessellator = FakeGhost::tessellator();
        let mut camera = FakeBuffer::new(size_of::<CameraLayout>());
        let mut screen = FakeBuffer::new(0x100);
        screen.put_u64(calls.screen_tessellator as usize, tessellator.as_mut_ptr() as u64);
        screen.put_u64(MeshContextOffset::BYTES + 8, camera.as_mut_ptr() as u64);
        Self {
            renderer: GhostRenderer::new(&calls),
            calls,
            ghost: SharedGhost::new_shared(),
            tessellator,
            screen,
            _camera: camera,
            atlas: Box::new(TexturePtrLayout::EMPTY),
        }
    }

    /// The world of `region`, with the rig's player, material and atlas.
    pub fn world(&self, region: usize) -> GhostWorld {
        GhostWorld {
            region: region as *mut _,
            feet: Self::FEET,
            blend: Self::BLEND as *const _,
            atlas: &*self.atlas as *const TexturePtrLayout,
        }
    }

    /// One render frame in `region`, or out of a world for `None`.
    pub fn frame(&mut self, region: Option<usize>) {
        let mut renderer = std::mem::replace(&mut self.renderer, GhostRenderer::new(&self.calls));
        self.frame_with(&mut renderer, region);
        self.renderer = renderer;
    }

    /// One render frame of `renderer` on the rig's fake objects, in `region` or out of a world for `None`.
    #[allow(unsafe_code)]
    pub fn frame_with(&mut self, renderer: &mut GhostRenderer, region: Option<usize>) {
        let world = region.map(|region| self.world(region));
        // SAFETY: the fake screen context is laid out as the mirrors and outlives the wrapper
        let screen = unsafe { ScreenContext::from_raw(self.screen.as_mut_ptr(), &self.calls) }.unwrap();
        renderer.run_frame(world.as_ref(), &screen, &self.ghost, [0.0; 3]);
    }

    /// Publishes `frame` as the core's tick does.
    pub fn publish(&self, frame: &GhostFrame) {
        self.ghost.publish(frame);
    }

    /// Every report the frames sent since the last call, oldest first.
    pub fn reports(&self) -> Vec<GhostReport> {
        let mut reports = Vec::new();
        self.ghost.drain(|report| reports.push(report.clone()));
        reports
    }

    /// A frame of `generation` in world `epoch`, with the rig's slot count and a palette of `entries`.
    pub fn ghost_frame(epoch: u64, generation: u64, entries: &[&str]) -> GhostFrame {
        let mut frame = GhostFrame::new();
        frame.generation = generation;
        frame.world_epoch = epoch;
        frame.in_world = true;
        frame.slot_capacity = Self::SLOTS;
        let mut palette = GhostPalette::new();
        for name in entries {
            palette.entries.push(BlockState::new((*name).to_string(), BlockStates::new(Vec::new())));
        }
        frame.palette = Some(Arc::new(palette));
        frame
    }

    /// A Replace job of one cell of palette id 0 at `origin`, into `slot`.
    pub fn job(slot: u32, serial: u32, id: u16, origin: [i32; 3]) -> GhostJob {
        let key = SectionKey { x: slot as i32, y: 0, z: 0, layer: None, part: SectionPart::Whole };
        let mut job = GhostJob::new(key);
        job.slot = slot;
        job.serial = serial;
        job.origin = origin;
        job.dims = [1, 1, 1];
        job.slot_size = [1, 1, 1];
        job.cells = vec![u16::MAX; 27];
        job.cells[13] = id;
        job
    }

    /// A group job of one cell of palette id 0 at `origin`: `mode` Append or Finish, at `index` of group `serial`.
    pub fn group_job(slot: u32, serial: u32, mode: GhostJobMode, group_serial: u32, index: u32) -> GhostJob {
        let mut job = Self::job(slot, serial, 0, [index as i32, 64, 0]);
        job.mode = mode;
        job.group_serial = group_serial;
        job.group_index = index;
        job.slot_size = [4, 1, 1];
        job
    }

    /// A draw of `slot` at `origin`, unscaled.
    pub fn draw(slot: u32, origin: [i32; 3]) -> GhostDraw {
        GhostDraw { slot, origin, offset: [0; 3], scale: 1.0 }
    }
}
