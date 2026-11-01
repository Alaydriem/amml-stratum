use std::sync::Arc;

use super::{BlockTexture, PreviewBounds, PreviewQuad};
use crate::structs::structure::StructureSize;

// The faces of one structure that can be seen, with the textures they use.
// Bound: at most `max_quads` quads (`PreviewConfig::max_quads`); a push past that is dropped and sets `truncated`.
#[derive(Debug, Clone, PartialEq)]
pub struct PreviewMesh {
    pub size: StructureSize,
    quads: Vec<PreviewQuad>,
    max_quads: usize,
    // The box around every quad; empty while there is none. Only `push` adds quads, so it never falls behind.
    bounds: PreviewBounds,
    // Index 0 is the grey stand-in for a missing texture.
    pub textures: Vec<Arc<BlockTexture>>,
    // Palette entries with no texture found.
    pub missing: u32,
    pub truncated: bool,
}

impl PreviewMesh {
    pub fn new(size: StructureSize, textures: Vec<Arc<BlockTexture>>, max_quads: usize) -> Self {
        Self {
            size,
            quads: Vec::new(),
            max_quads,
            bounds: PreviewBounds::empty(),
            textures,
            missing: 0,
            truncated: false,
        }
    }

    // Adds `quad` and grows the bounds to hold it. A full mesh drops the quad and is marked truncated.
    pub fn push(&mut self, quad: PreviewQuad) {
        if self.quads.len() >= self.max_quads {
            self.truncated = true;
            return;
        }
        for corner in quad.corners {
            self.bounds.include(corner);
        }
        self.quads.push(quad);
    }

    // True when `count` more quads fit.
    pub fn has_room(&self, count: usize) -> bool {
        self.quads.len().saturating_add(count) <= self.max_quads
    }

    pub fn quads(&self) -> &[PreviewQuad] {
        &self.quads
    }

    // The box around the quads; empty when there is none.
    pub fn bounds(&self) -> PreviewBounds {
        self.bounds
    }

    // The box the camera fits: the drawn quads, or the structure's box when nothing is drawn.
    pub fn view_bounds(&self) -> PreviewBounds {
        if self.bounds.is_empty() { PreviewBounds::of_size(self.size) } else { self.bounds }
    }

    pub fn triangles(&self) -> usize {
        self.quads.len() * 2
    }
}
