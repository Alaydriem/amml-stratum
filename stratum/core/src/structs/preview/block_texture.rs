// One decoded block texture: the first frame only, as straight RGBA rows from the top.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockTexture {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

impl BlockTexture {
    // None when `rgba` does not hold `width * height` pixels or a side is zero.
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Option<Self> {
        let pixels = (width as usize).checked_mul(height as usize)?;
        if width == 0 || height == 0 || rgba.len() != pixels.checked_mul(4)? {
            return None;
        }
        Some(Self { width, height, rgba })
    }

    // A one pixel texture of one colour, for a block whose texture is missing.
    pub fn solid(rgba: [u8; 4]) -> Self {
        Self { width: 1, height: 1, rgba: rgba.to_vec() }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }

    // The nearest texel to `u`, `v` in 0..1, clamped to the edge.
    #[inline]
    pub fn sample(&self, u: f32, v: f32) -> [u8; 4] {
        let x = ((u * self.width as f32) as i64).clamp(0, i64::from(self.width) - 1) as usize;
        let y = ((v * self.height as f32) as i64).clamp(0, i64::from(self.height) - 1) as usize;
        let at = (y * self.width as usize + x) * 4;
        match self.rgba.get(at..at + 4) {
            Some(&[r, g, b, a]) => [r, g, b, a],
            _ => [0, 0, 0, 0],
        }
    }

    // Heap memory the pixels hold.
    pub fn heap_bytes(&self) -> u64 {
        self.rgba.len() as u64
    }
}
