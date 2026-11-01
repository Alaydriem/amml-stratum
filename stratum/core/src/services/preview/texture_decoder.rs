use std::io::Cursor;

use crate::structs::preview::{BlockTexture, TextureTint};

// Turns the PNG and TGA files of the game's block textures into RGBA texels. An animated texture (a strip of
// square frames, taller than wide) keeps its first frame.
pub struct TextureDecoder;

impl TextureDecoder {
    // The largest side read; bigger files are refused.
    pub const MAX_SIDE: u32 = 4096;

    pub fn decode(bytes: &[u8], tga: bool, tint: TextureTint) -> Option<BlockTexture> {
        let (width, height, mut rgba) = if tga { Self::tga(bytes)? } else { Self::png(bytes)? };
        let frame = if height > width && height % width == 0 { width } else { height };
        rgba.truncate((width as usize) * (frame as usize) * 4);
        tint.apply(&mut rgba);
        BlockTexture::new(width, frame, rgba)
    }

    pub fn png(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
        let mut decoder = png::Decoder::new(Cursor::new(bytes));
        decoder.set_transformations(png::Transformations::normalize_to_color8());
        let mut reader = decoder.read_info().ok()?;
        let mut buffer = vec![0; reader.output_buffer_size()?];
        let info = reader.next_frame(&mut buffer).ok()?;
        if info.width == 0 || info.height == 0 || info.width > Self::MAX_SIDE || info.height > Self::MAX_SIDE {
            return None;
        }
        let pixels = (info.width as usize) * (info.height as usize);
        let channels = match info.color_type {
            png::ColorType::Grayscale => 1,
            png::ColorType::GrayscaleAlpha => 2,
            png::ColorType::Rgb => 3,
            png::ColorType::Rgba => 4,
            png::ColorType::Indexed => return None,
        };
        let source = buffer.get(..pixels * channels)?;
        let mut rgba = Vec::with_capacity(pixels * 4);
        for texel in source.chunks_exact(channels) {
            let rgba_texel = match *texel {
                [g] => [g, g, g, 255],
                [g, a] => [g, g, g, a],
                [r, g, b] => [r, g, b, 255],
                [r, g, b, a] => [r, g, b, a],
                _ => return None,
            };
            rgba.extend_from_slice(&rgba_texel);
        }
        Some((info.width, info.height, rgba))
    }

    // Uncompressed (type 2) and run length (type 10) true colour TGA files, 24 or 32 bits, either row order.
    pub fn tga(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
        let header = bytes.get(..18)?;
        let id_len = usize::from(header[0]);
        let colour_map = header[1];
        let kind = header[2];
        let map_len = usize::from(u16::from_le_bytes([header[5], header[6]]));
        let map_bits = usize::from(header[7]);
        let width = u32::from(u16::from_le_bytes([header[12], header[13]]));
        let height = u32::from(u16::from_le_bytes([header[14], header[15]]));
        let bits = header[16];
        let top_down = header[17] & 0x20 != 0;
        if (kind != 2 && kind != 10) || (bits != 24 && bits != 32) || width == 0 || height == 0 {
            return None;
        }
        if width > Self::MAX_SIDE || height > Self::MAX_SIDE {
            return None;
        }
        let map_bytes = if colour_map == 1 { map_len * map_bits.div_ceil(8) } else { 0 };
        let mut at = 18 + id_len + map_bytes;
        let step = usize::from(bits / 8);
        let pixels = (width as usize) * (height as usize);
        let mut bgra: Vec<[u8; 4]> = Vec::with_capacity(pixels);
        let read = |at: usize| -> Option<[u8; 4]> {
            let texel = bytes.get(at..at + step)?;
            Some([texel[0], texel[1], texel[2], if step == 4 { texel[3] } else { 255 }])
        };
        while bgra.len() < pixels {
            if kind == 2 {
                bgra.push(read(at)?);
                at += step;
                continue;
            }
            let packet = *bytes.get(at)?;
            at += 1;
            let run = usize::from(packet & 0x7f) + 1;
            if packet & 0x80 != 0 {
                let texel = read(at)?;
                at += step;
                for _ in 0..run.min(pixels - bgra.len()) {
                    bgra.push(texel);
                }
            } else {
                for _ in 0..run.min(pixels - bgra.len()) {
                    bgra.push(read(at)?);
                    at += step;
                }
            }
        }
        let row = width as usize;
        let mut rgba = Vec::with_capacity(pixels * 4);
        for y in 0..height as usize {
            let source_row = if top_down { y } else { height as usize - 1 - y };
            for texel in bgra.get(source_row * row..(source_row + 1) * row)? {
                rgba.extend_from_slice(&[texel[2], texel[1], texel[0], texel[3]]);
            }
        }
        Some((width, height, rgba))
    }
}
