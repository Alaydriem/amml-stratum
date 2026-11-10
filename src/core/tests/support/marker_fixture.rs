use stratum_core::config::MarkerConfig;
use stratum_core::services::MarkerService;
use stratum_core::structs::marker::{MarkerSet, WorldKey};

pub struct MarkerFixture;

impl MarkerFixture {
    pub fn world() -> WorldKey {
        WorldKey { world: "Test World".to_string(), dimension: 0 }
    }

    pub fn service(max_per_world: usize) -> MarkerService {
        MarkerService::new(MarkerSet::new(Self::world()), MarkerConfig { max_per_world })
    }
}
