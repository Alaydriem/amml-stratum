use stratum_core::structs::ghost::{GhostDraw, GhostJob};

use super::CoreSession;

// One ghost take, copied out of the take's buffers so a test can read it after the next take. Each job carries the
// generation of the frame that gave it.
#[derive(Debug, Clone)]
pub struct GhostSeen {
    // the take was made; false after a panic poisoned it or while another take was in progress
    pub taken: bool,
    pub generation: u64,
    pub world_epoch: u64,
    pub in_world: bool,
    pub clear_all: bool,
    pub draws: Vec<GhostDraw>,
    pub jobs: Vec<(u64, GhostJob)>,
    pub frees: Vec<u32>,
    // The NBT bytes of each entry, when the palette changed.
    pub palette: Option<Vec<Vec<u8>>>,
    pub opacity: f32,
    pub rotation: u8,
}

impl GhostSeen {
    // The first job that builds a full ghost section: the mini ghost and the marker skip no matching blocks.
    pub fn full_job(&self) -> &GhostJob {
        self.jobs.iter().map(|(_, job)| job).find(|job| job.skip_matching).expect("the take holds a full ghost job")
    }

    // The generation of the full ghost job.
    pub fn full_job_generation(&self) -> u64 {
        self.jobs.iter().find(|(_, job)| job.skip_matching).map(|(generation, _)| *generation).unwrap()
    }

    // The draws at scale 1.0: the full ghost, and the marker when it is drawn.
    pub fn unscaled_draws(&self) -> Vec<GhostDraw> {
        self.draws.iter().filter(|draw| draw.scale == 1.0).copied().collect()
    }

    // The draws below scale 1.0: the mini ghost.
    pub fn miniature_draws(&self) -> Vec<GhostDraw> {
        self.draws.iter().filter(|draw| draw.scale < 1.0).copied().collect()
    }

    pub fn take(session: &CoreSession) -> Self {
        let Some(take) = session.handle().ghost().take() else {
            return Self {
                taken: false,
                generation: 0,
                world_epoch: 0,
                in_world: false,
                clear_all: false,
                draws: Vec::new(),
                jobs: Vec::new(),
                frees: Vec::new(),
                palette: None,
                opacity: 0.0,
                rotation: 0,
            };
        };
        Self {
            taken: true,
            generation: take.generation(),
            world_epoch: take.world_epoch(),
            in_world: take.in_world(),
            clear_all: take.clear_all(),
            draws: take.draws().to_vec(),
            jobs: take.jobs().to_vec(),
            frees: take.frees().to_vec(),
            palette: take.palette().map(|entries| entries.map(<[u8]>::to_vec).collect()),
            opacity: take.opacity(),
            rotation: take.rotation(),
        }
    }
}
