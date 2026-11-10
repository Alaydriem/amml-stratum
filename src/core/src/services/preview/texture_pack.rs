use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::Value;

use super::{BrArchive, JsonText, TextureDecoder};
use crate::structs::block::Face;
use crate::structs::preview::{BlockTexture, TextureTint};

// The block textures of the installed game, read once from its resource packs. The game layers `vanilla` and every
// `vanilla_<version>` pack in version order, a later pack replacing what an earlier one names; this does the same
// for `blocks.json`, `textures/terrain_texture.json` and the block texture archives under `__brarchive`.
// Bound: the archive bytes are what the packs hold (about 7 MB for the game version it was written against), and
// at most `capacity` decoded textures are kept (`PreviewConfig::texture_cache`); a decode past that empties the
// cache first.
pub struct TexturePack {
    // Block name without the namespace, to the texture key of each face in `Face` order.
    blocks: HashMap<String, [String; 6]>,
    // Texture key to the texture path (for example `textures/blocks/log_oak`) and whether it is an overlay texture.
    terrain: HashMap<String, (String, bool)>,
    archives: Vec<Vec<u8>>,
    // Texture path without the extension to (archive, start, length, is TGA).
    files: HashMap<String, (usize, usize, usize, bool)>,
    cache: HashMap<String, Option<Arc<BlockTexture>>>,
    capacity: usize,
    packs: usize,
}

impl TexturePack {
    // Block names the packs do not list under their own name.
    const ALIASES: [(&'static str, &'static str); 1] = [("grass_block", "grass")];

    // A pack with no textures: every block draws the grey stand-in. It keeps at most `capacity` textures, at
    // least one.
    pub fn empty(capacity: usize) -> Self {
        Self {
            blocks: HashMap::new(),
            terrain: HashMap::new(),
            archives: Vec::new(),
            files: HashMap::new(),
            cache: HashMap::new(),
            packs: 0,
            capacity: capacity.max(1),
        }
    }

    // Reads the packs under `packs_dir`. A missing or broken file is skipped, so this never fails; a folder with
    // no packs gives an empty pack. It keeps at most `capacity` decoded textures.
    pub fn load(packs_dir: &Path, capacity: usize) -> Self {
        Self::load_until(packs_dir, capacity, || false)
    }

    // Like `load`, giving back the packs read so far as soon as `stopped` returns true; it is asked before each
    // pack and each texture archive.
    pub fn load_until(packs_dir: &Path, capacity: usize, stopped: impl Fn() -> bool) -> Self {
        let mut pack = Self::empty(capacity);
        for dir in Self::pack_dirs(packs_dir) {
            if stopped() {
                return pack;
            }
            pack.packs += 1;
            pack.read_blocks(&dir.join("blocks.json"));
            pack.read_terrain(&dir.join("textures").join("terrain_texture.json"));
            let textures = dir.join("__brarchive").join("textures");
            pack.read_archive(&textures.join("blocks.brarchive"), "textures/blocks");
            if let Ok(listing) = std::fs::read_dir(textures.join("blocks")) {
                let mut nested: Vec<PathBuf> =
                    listing.filter_map(|entry| entry.ok().map(|entry| entry.path())).collect();
                nested.sort();
                for path in nested {
                    if stopped() {
                        return pack;
                    }
                    if path.extension().is_some_and(|ext| ext == "brarchive")
                        && let Some(stem) = path.file_stem().and_then(|stem| stem.to_str())
                    {
                        let prefix = format!("textures/blocks/{stem}");
                        pack.read_archive(&path, &prefix);
                    }
                }
            }
        }
        pack
    }

    pub fn pack_count(&self) -> usize {
        self.packs
    }

    // Texture files the archives hold.
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    // Changes the cache bound; a cache already past the new bound is emptied.
    pub fn set_capacity(&mut self, capacity: usize) {
        self.capacity = capacity.max(1);
        if self.cache.len() > self.capacity {
            self.cache.clear();
        }
    }

    pub fn cached(&self) -> usize {
        self.cache.len()
    }

    pub fn archive_bytes(&self) -> u64 {
        self.archives.iter().map(|archive| archive.len() as u64).sum()
    }

    // The texture key of each face for a block (`minecraft:` is dropped), or None when no pack names the block
    // and no texture carries its name.
    pub fn face_keys(&self, block: &str) -> Option<[String; 6]> {
        let name = block.strip_prefix("minecraft:").unwrap_or(block);
        let name = Self::ALIASES.iter().find(|(from, _)| *from == name).map_or(name, |(_, to)| *to);
        if let Some(keys) = self.blocks.get(name) {
            return Some(keys.clone());
        }
        if self.terrain.contains_key(name) || self.files.contains_key(&format!("textures/blocks/{name}")) {
            return Some(std::array::from_fn(|_| name.to_owned()));
        }
        None
    }

    // The decoded texture for a key from `face_keys`, tinted for the preview; None when it cannot be found or read.
    pub fn texture(&mut self, key: &str) -> Option<Arc<BlockTexture>> {
        if let Some(found) = self.cache.get(key) {
            return found.clone();
        }
        let decoded = self.decode(key).map(Arc::new);
        if self.cache.len() >= self.capacity {
            self.cache.clear();
        }
        self.cache.insert(key.to_owned(), decoded.clone());
        decoded
    }

    // Puts a texture under a key, for a pack built without files.
    pub fn insert(&mut self, block: &str, keys: [String; 6], textures: Vec<(String, BlockTexture)>) {
        self.blocks.insert(block.to_owned(), keys);
        for (key, texture) in textures {
            if self.cache.len() >= self.capacity {
                self.cache.clear();
            }
            self.cache.insert(key, Some(Arc::new(texture)));
        }
    }

    fn decode(&self, key: &str) -> Option<BlockTexture> {
        let (path, overlay) = match self.terrain.get(key) {
            Some((path, overlay)) => (path.clone(), *overlay),
            None => (format!("textures/blocks/{key}"), false),
        };
        let (archive, start, len, tga) = *self.files.get(&path)?;
        let bytes = self.archives.get(archive)?.get(start..start.checked_add(len)?)?;
        TextureDecoder::decode(bytes, tga, TextureTint::for_path(&path, overlay))
    }

    // `vanilla` first, then each `vanilla_<version>` in version order.
    fn pack_dirs(packs_dir: &Path) -> Vec<PathBuf> {
        let Ok(listing) = std::fs::read_dir(packs_dir) else {
            return Vec::new();
        };
        let mut packs: Vec<(Vec<u32>, PathBuf)> = listing
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let name = entry.file_name().to_str()?.to_owned();
                let version = if name == "vanilla" {
                    Vec::new()
                } else {
                    let digits = name.strip_prefix("vanilla_")?;
                    digits.split('.').map(|part| part.parse::<u32>().ok()).collect::<Option<Vec<u32>>>()?
                };
                Some((version, entry.path()))
            })
            .collect();
        packs.sort();
        packs.into_iter().map(|(_, path)| path).collect()
    }

    fn read_json(path: &Path) -> Option<Value> {
        let text = std::fs::read_to_string(path).ok()?;
        serde_json::from_str(&JsonText::strip_comments(&text)).ok()
    }

    fn read_blocks(&mut self, path: &Path) {
        let Some(Value::Object(blocks)) = Self::read_json(path) else {
            return;
        };
        for (name, entry) in blocks {
            if let Some(keys) = entry.get("textures").and_then(Self::faces) {
                self.blocks.insert(name, keys);
            }
        }
    }

    // A `textures` value: one key for every face, or keys by face with `side` for the four sides.
    fn faces(textures: &Value) -> Option<[String; 6]> {
        if let Some(all) = textures.as_str() {
            return Some(std::array::from_fn(|_| all.to_owned()));
        }
        let get = |key: &str| textures.get(key).and_then(Value::as_str);
        let side = get("side").or_else(|| get("north")).or_else(|| get("up"))?;
        let mut keys: [String; 6] = std::array::from_fn(|_| side.to_owned());
        for face in Face::ALL {
            if let (Some(key), Some(slot)) = (get(face.json_key()), keys.get_mut(face.index())) {
                *slot = key.to_owned();
            }
        }
        Some(keys)
    }

    fn read_terrain(&mut self, path: &Path) {
        let Some(json) = Self::read_json(path) else {
            return;
        };
        let Some(Value::Object(data)) = json.get("texture_data") else {
            return;
        };
        for (key, entry) in data {
            let first = match entry.get("textures") {
                Some(Value::Array(variants)) => variants.first(),
                other => other,
            };
            let found = match first {
                Some(Value::String(path)) => Some((path.clone(), false)),
                Some(Value::Object(variant)) => variant
                    .get("path")
                    .and_then(Value::as_str)
                    .map(|path| (path.to_owned(), variant.contains_key("overlay_color"))),
                _ => None,
            };
            if let Some(found) = found {
                self.terrain.insert(key.clone(), found);
            }
        }
    }

    fn read_archive(&mut self, path: &Path, prefix: &str) {
        let Ok(bytes) = std::fs::read(path) else {
            return;
        };
        let Some(entries) = BrArchive::entries(&bytes) else {
            curia::warn!("texture archive could not be read", { file: path.display().to_string() });
            return;
        };
        let archive = self.archives.len();
        for entry in entries {
            let (stem, tga) = match entry.name.rsplit_once('.') {
                Some((stem, "png")) => (stem, false),
                Some((stem, "tga")) => (stem, true),
                _ => continue,
            };
            let key = format!("{prefix}/{stem}");
            // the game reads the TGA when an archive holds both, for its alpha
            let keep_old = self.files.get(&key).is_some_and(|old| old.0 == archive && old.3 && !tga);
            if !keep_old {
                self.files.insert(key, (archive, entry.start, entry.len, tga));
            }
        }
        self.archives.push(bytes);
    }
}
