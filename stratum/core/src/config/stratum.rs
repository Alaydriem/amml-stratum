use super::{
    CacheConfig, DebugConfig, EasyPlaceConfig, GhostConfig, ImportSettings, LibraryConfig, MarkerConfig, PreviewConfig,
};

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize)]
#[serde(default)]
pub struct StratumConfig {
    pub library: LibraryConfig,
    pub markers: MarkerConfig,
    pub import: ImportSettings,
    pub cache: CacheConfig,
    pub ghost: GhostConfig,
    pub easy_place: EasyPlaceConfig,
    pub preview: PreviewConfig,
    pub debug: DebugConfig,
}
