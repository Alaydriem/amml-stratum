#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(default)]
pub struct MarkerConfig {
    pub max_per_world: usize,
}

impl Default for MarkerConfig {
    fn default() -> Self {
        Self { max_per_world: 256 }
    }
}
