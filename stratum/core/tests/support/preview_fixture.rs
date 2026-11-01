use std::path::Path;

use stratum_core::config::PreviewConfig;
use stratum_core::services::preview::{BrArchive, PreviewMesher, TexturePack};
use stratum_core::structs::preview::{PreviewMesh, PreviewStop};
use stratum_core::structs::structure::{BlockLayer, BlockState, SparseLayer, Structure, StructureSize};

// Builds structures, texture archives, TGA files and resource pack folders for the preview tests.
pub struct PreviewFixture;

impl PreviewFixture {
    // A structure of `size` with `block` in every cell, built without a per-cell list.
    pub fn solid(size: [u32; 3], block: BlockState) -> Structure {
        let cells = vec![0u16; (size[0] * size[1] * size[2]) as usize];
        Structure {
            size: StructureSize::new(size[0], size[1], size[2]),
            world_origin: None,
            palette: vec![block],
            primary: BlockLayer::new(cells),
            secondary: SparseLayer::default(),
        }
    }

    // A mesher with the default bounds and a stop nobody sets.
    pub fn mesher() -> PreviewMesher {
        PreviewMesher::new(PreviewConfig::default().max_quads(), PreviewStop::new())
    }

    // A pack with no textures and the default cache bound.
    pub fn no_textures() -> TexturePack {
        TexturePack::empty(PreviewConfig::default().texture_cache())
    }

    // The mesh of `structure` with the default bounds and no textures.
    pub fn mesh(structure: &Structure) -> PreviewMesh {
        Self::mesher().build(structure, &mut Self::no_textures()).unwrap()
    }

    // A structure of the given size with each listed block in its cell; every other cell holds no block.
    pub fn structure(size: [u32; 3], blocks: &[([u32; 3], BlockState)]) -> Structure {
        let mut palette: Vec<BlockState> = Vec::new();
        let mut cells = vec![BlockLayer::NONE; (size[0] * size[1] * size[2]) as usize];
        for (at, block) in blocks {
            let index = match palette.iter().position(|known| known == block) {
                Some(index) => index,
                None => {
                    palette.push(block.clone());
                    palette.len() - 1
                }
            };
            let cell = ((at[0] * size[1] + at[1]) * size[2] + at[2]) as usize;
            cells[cell] = index as u16;
        }
        Structure {
            size: StructureSize::new(size[0], size[1], size[2]),
            world_origin: None,
            palette,
            primary: BlockLayer::new(cells),
            secondary: SparseLayer::default(),
        }
    }

    // `.brarchive` bytes holding the named files.
    pub fn archive(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let mut bytes = BrArchive::MAGIC.to_vec();
        bytes.extend_from_slice(&(files.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        let mut offset = 0u32;
        for (name, data) in files {
            let mut record = vec![0u8; 256];
            record[0] = name.len() as u8;
            record[1..1 + name.len()].copy_from_slice(name.as_bytes());
            record[248..252].copy_from_slice(&offset.to_le_bytes());
            record[252..256].copy_from_slice(&(data.len() as u32).to_le_bytes());
            bytes.extend_from_slice(&record);
            offset += data.len() as u32;
        }
        for (_, data) in files {
            bytes.extend_from_slice(data);
        }
        bytes
    }

    // An uncompressed 32 bit TGA, rows stored bottom first as the game's files are. `rows` are RGBA, top first.
    pub fn tga(width: u16, rows: &[Vec<[u8; 4]>]) -> Vec<u8> {
        let mut bytes = vec![0u8; 18];
        bytes[2] = 2;
        bytes[12..14].copy_from_slice(&width.to_le_bytes());
        bytes[14..16].copy_from_slice(&(rows.len() as u16).to_le_bytes());
        bytes[16] = 32;
        bytes[17] = 8;
        for row in rows.iter().rev() {
            for [r, g, b, a] in row {
                bytes.extend_from_slice(&[*b, *g, *r, *a]);
            }
        }
        bytes
    }

    // A `vanilla` pack under `dir` with a `blocks.json` (with a line comment), a terrain list and one archive.
    pub fn pack(dir: &Path, blocks_json: &str, terrain_json: &str, files: &[(&str, Vec<u8>)]) {
        let pack = dir.join("vanilla");
        std::fs::create_dir_all(pack.join("textures")).unwrap();
        std::fs::create_dir_all(pack.join("__brarchive").join("textures")).unwrap();
        std::fs::write(pack.join("blocks.json"), blocks_json).unwrap();
        std::fs::write(pack.join("textures").join("terrain_texture.json"), terrain_json).unwrap();
        std::fs::write(pack.join("__brarchive").join("textures").join("blocks.brarchive"), Self::archive(files))
            .unwrap();
    }
}
