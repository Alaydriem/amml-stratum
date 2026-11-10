// The colour a grey game texture is given for the preview. The game takes it from the biome; the preview uses
// fixed plains colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureTint {
    None,
    // Every texel is multiplied by the colour.
    Multiply([u8; 3]),
    // Texels the alpha channel marks are multiplied by the colour, and every texel turns opaque. The game's
    // `overlay_color` textures (the grass side) work this way.
    Overlay([u8; 3]),
    // Multiplied by the colour and made opaque (water).
    Liquid([u8; 3]),
}

impl TextureTint {
    pub const PLAINS_GRASS: [u8; 3] = [0x91, 0xbd, 0x59];
    pub const PLAINS_FOLIAGE: [u8; 3] = [0x77, 0xab, 0x2f];
    pub const SPRUCE_FOLIAGE: [u8; 3] = [0x61, 0x99, 0x61];
    pub const BIRCH_FOLIAGE: [u8; 3] = [0x80, 0xa7, 0x55];
    pub const MANGROVE_FOLIAGE: [u8; 3] = [0x8d, 0xb1, 0x27];
    pub const PLAINS_WATER: [u8; 3] = [0x44, 0xaf, 0xf5];

    // The tint for the texture at `path` (for example `textures/blocks/leaves_oak`). `overlay` is true when the
    // game's terrain list gives the texture an `overlay_color`.
    pub fn for_path(path: &str, overlay: bool) -> Self {
        let name = path.rsplit('/').next().unwrap_or(path);
        if overlay {
            return Self::Overlay(Self::PLAINS_GRASS);
        }
        if name.ends_with("_opaque") || name.ends_with("_carried") {
            return Self::None;
        }
        match name {
            "leaves_spruce" => Self::Multiply(Self::SPRUCE_FOLIAGE),
            "leaves_birch" => Self::Multiply(Self::BIRCH_FOLIAGE),
            "mangrove_leaves" => Self::Multiply(Self::MANGROVE_FOLIAGE),
            "leaves_oak" | "leaves_jungle" | "leaves_acacia" | "leaves_big_oak" | "vine" => {
                Self::Multiply(Self::PLAINS_FOLIAGE)
            }
            "grass_top" | "tallgrass" | "fern" | "waterlily" => Self::Multiply(Self::PLAINS_GRASS),
            _ if name.starts_with("double_plant_grass") || name.starts_with("double_plant_fern") => {
                Self::Multiply(Self::PLAINS_GRASS)
            }
            _ if name.contains("water") && name.ends_with("_grey") => Self::Liquid(Self::PLAINS_WATER),
            _ => Self::None,
        }
    }

    // Applies the tint to RGBA texels in place.
    pub fn apply(self, rgba: &mut [u8]) {
        for texel in rgba.as_chunks_mut::<4>().0 {
            match self {
                Self::None => return,
                Self::Multiply(colour) => Self::multiply(texel, colour),
                Self::Overlay(colour) => {
                    if texel[3] > 0 {
                        Self::multiply(texel, colour);
                    }
                    texel[3] = 255;
                }
                Self::Liquid(colour) => {
                    Self::multiply(texel, colour);
                    texel[3] = 255;
                }
            }
        }
    }

    fn multiply(texel: &mut [u8], colour: [u8; 3]) {
        for (channel, tint) in texel.iter_mut().zip(colour) {
            *channel = ((u16::from(*channel) * u16::from(tint) + 127) / 255) as u8;
        }
    }
}
