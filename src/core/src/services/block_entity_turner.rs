use crate::services::{NbtReader, NbtWriter};
use crate::structs::ghost::ChestPairNbt;
use crate::structs::marker::Rotation;
use crate::structs::nbt::{NbtCompound, NbtLimits, NbtTag};

// Turns the directions held in block entity NBT with a ghost's rotation. Block-state directions are
// not here; the mod turns those when it resolves a block.
pub struct BlockEntityTurner;

impl BlockEntityTurner {
    // Bedrock `facing` values of a shulker box in clockwise order seen from above: north, east, south,
    // west. Down (0) and up (1) do not turn.
    const SHULKER_RING: [i8; 4] = [2, 5, 3, 4];

    // The NBT of the cell at `cell` turned by `rotation`, or None when nothing in it turns and the
    // bytes stay as they are. `turned_cell` is where the cell sits in the turned structure. A chest's
    // `pairx` / `pairz` are world coordinates when `world_origin` is set, local ones when it is None;
    // the turned NBT keeps that frame, so it reads the same at every rotation.
    pub fn turn(
        nbt: &[u8],
        rotation: Rotation,
        cell: [i32; 3],
        turned_cell: [i32; 3],
        world_origin: Option<[i32; 3]>,
    ) -> Option<Vec<u8>> {
        let (name, root) = NbtReader::read(nbt, NbtLimits::STRUCTURE).ok()?;
        let mut entries: Vec<(&str, NbtTag)> = root.iter().map(|(key, tag)| (key, tag.clone())).collect();
        let mut changed = false;
        let shulker = matches!(root.get("id"), Some(NbtTag::String(id)) if id == "ShulkerBox");
        let pair = Self::pair_of(&root);
        for (key, tag) in entries.iter_mut() {
            match (*key, &mut *tag) {
                ("facing", NbtTag::Byte(facing)) if shulker => {
                    let turned = Self::turn_facing(*facing, rotation);
                    changed |= turned != *facing;
                    *facing = turned;
                }
                ("pairx", NbtTag::Int(value)) | ("pairz", NbtTag::Int(value)) if pair.is_some() => {
                    let (pair_x, pair_z) = pair?;
                    let origin = world_origin.unwrap_or([0; 3]);
                    let offset = rotation.turn_offset([
                        i64::from(pair_x) - i64::from(origin[0]) - i64::from(cell[0]),
                        0,
                        i64::from(pair_z) - i64::from(origin[2]) - i64::from(cell[2]),
                    ]);
                    let turned = if *key == "pairx" {
                        i64::from(origin[0]) + i64::from(turned_cell[0]) + offset[0]
                    } else {
                        i64::from(origin[2]) + i64::from(turned_cell[2]) + offset[2]
                    };
                    let turned = i32::try_from(turned).ok()?;
                    changed |= turned != *value;
                    *value = turned;
                }
                _ => {}
            }
        }
        if !changed {
            return None;
        }
        NbtWriter::compound(&name, &entries).ok()
    }

    // The offset from the cell at local position `cell` to the other half of its double chest, or None
    // when the NBT names no pair. The pair is a world position when `world_origin` is set, a local one
    // when it is None.
    pub fn chest_pair_offset(nbt: &[u8], cell: [i32; 3], world_origin: Option<[i32; 3]>) -> Option<[i32; 3]> {
        let (_, root) = NbtReader::read(nbt, NbtLimits::STRUCTURE).ok()?;
        let (pair_x, pair_z) = Self::pair_of(&root)?;
        ChestPairNbt::new(pair_x, pair_z, false).offset(cell, world_origin)
    }

    // The NBT without its double chest keys (`pairlead`, `pairx`, `pairz`), or None when it has none. The game
    // reads those keys as a chest that is paired, and a ghost chest draws its pair from the mod instead.
    pub fn strip_pair(nbt: &[u8]) -> Option<Vec<u8>> {
        Self::split_pair(nbt).map(|(stripped, _)| stripped)
    }

    // The NBT without its double chest keys and the pair those keys name, from one read; None when it has no
    // double chest key. The pair is None when `pairx` or `pairz` is missing or not an Int.
    pub fn split_pair(nbt: &[u8]) -> Option<(Vec<u8>, Option<ChestPairNbt>)> {
        if !nbt.windows(4).any(|window| window == b"pair") {
            return None;
        }
        let (name, root) = NbtReader::read(nbt, NbtLimits::STRUCTURE).ok()?;
        let lead = matches!(root.get("pairlead"), Some(NbtTag::Byte(1)));
        let pair = Self::pair_of(&root).map(|(pair_x, pair_z)| ChestPairNbt::new(pair_x, pair_z, lead));
        let kept: Vec<(&str, NbtTag)> = root
            .iter()
            .filter(|(key, _)| !matches!(*key, "pairlead" | "pairx" | "pairz"))
            .map(|(key, tag)| (key, tag.clone()))
            .collect();
        if kept.len() == root.len() {
            return None;
        }
        Some((NbtWriter::compound(&name, &kept).ok()?, pair))
    }

    // True when the turned NBT depends on where the cell is, so one result cannot serve every cell
    // that holds this NBT.
    pub fn depends_on_cell(nbt: &[u8]) -> bool {
        NbtReader::read(nbt, NbtLimits::STRUCTURE).is_ok_and(|(_, root)| Self::pair_of(&root).is_some())
    }

    fn pair_of(root: &NbtCompound) -> Option<(i32, i32)> {
        match (root.get("pairx"), root.get("pairz")) {
            (Some(NbtTag::Int(x)), Some(NbtTag::Int(z))) => Some((*x, *z)),
            _ => None,
        }
    }

    fn turn_facing(facing: i8, rotation: Rotation) -> i8 {
        match Self::SHULKER_RING.iter().position(|value| *value == facing) {
            Some(at) => Self::SHULKER_RING[(at + usize::from(rotation.quarters())) % 4],
            None => facing,
        }
    }
}
