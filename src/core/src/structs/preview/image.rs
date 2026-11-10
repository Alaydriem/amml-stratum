use super::PreviewStats;

// A finished picture: straight RGBA rows from the top, `width * height` pixels.
#[derive(Debug, Clone, PartialEq)]
pub struct PreviewImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
    pub stats: PreviewStats,
}
