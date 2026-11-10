// The `preview` block of the config file: the memory bounds of the 3D structure preview. Both apply to the next
// picture after a change.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(default)]
pub struct PreviewConfig {
    // Decoded block textures the texture pack keeps; a decode past this empties the cache first.
    pub texture_cache: usize,
    // Quads one preview mesh holds; a structure with more is cut.
    pub max_quads: usize,
}

impl PreviewConfig {
    // The bounds of each value. The ceilings are the maxima of the settings fields in the descriptor; the quad floor
    // is the 12 quads of one stairs block.
    pub const MIN_TEXTURE_CACHE: usize = 1;
    pub const MAX_TEXTURE_CACHE: usize = 8192;
    pub const MIN_MAX_QUADS: usize = 12;
    pub const MAX_MAX_QUADS: usize = 4_000_000;

    pub fn texture_cache(&self) -> usize {
        self.texture_cache.clamp(Self::MIN_TEXTURE_CACHE, Self::MAX_TEXTURE_CACHE)
    }

    pub fn max_quads(&self) -> usize {
        self.max_quads.clamp(Self::MIN_MAX_QUADS, Self::MAX_MAX_QUADS)
    }
}

impl Default for PreviewConfig {
    fn default() -> Self {
        Self { texture_cache: 1024, max_quads: 1_000_000 }
    }
}
