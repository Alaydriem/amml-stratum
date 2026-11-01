use std::borrow::Cow;

use crate::errors::StructureError;
use crate::services::NbtReader;
use crate::structs::nbt::{NbtCompound, NbtLimits, NbtTag};
use crate::structs::structure::{
    BlockLayer, BlockState, BlockStates, SparseLayer, StateValue, Structure, StructureSize,
};

// Turns `.mcstructure` bytes into a `Structure`.
pub struct StructureParser;

impl StructureParser {
    // 256 x 384 x 256 cells.
    const MAX_VOLUME: u64 = 256 * 384 * 256;
    // Palette indices are stored as u16 with u16::MAX meaning no block, so this many entries
    // or more cannot be stored.
    const MAX_PALETTE: usize = BlockLayer::NONE as usize;
    // Cells read between two checks of the stop in `parse_until`.
    const STOP_EVERY: usize = 1 << 16;

    pub fn parse(bytes: &[u8]) -> Result<Structure, StructureError> {
        Self::parse_until(bytes, || false)
    }

    // Like `parse`, ending with `Stopped` as soon as `stopped` returns true; it is asked once per `STOP_EVERY` cells
    // of each block layer.
    pub fn parse_until(bytes: &[u8], stopped: impl Fn() -> bool) -> Result<Structure, StructureError> {
        let (_, root) = NbtReader::read(bytes, NbtLimits::STRUCTURE)?;

        let size = Self::read_size(&root)?;
        let volume = size.volume();
        // Checked before any index list is read, so an oversize file fails here.
        if volume > Self::MAX_VOLUME {
            return Err(StructureError::TooLarge { volume });
        }
        let world_origin = Self::read_origin(&root);

        let structure = Self::compound(&root, "structure")?;
        let palette = Self::read_palette(structure)?;
        if palette.len() >= Self::MAX_PALETTE {
            return Err(StructureError::PaletteTooLarge { entries: palette.len() });
        }
        let [primary, secondary] = Self::list(structure, "block_indices")? else {
            return Err(StructureError::MissingField("block_indices"));
        };
        let primary = Self::dense_layer(&Self::layer_bytes(bytes, primary, volume)?, palette.len(), &stopped)?;
        let secondary = Self::sparse_layer(&Self::layer_bytes(bytes, secondary, volume)?, &palette, &stopped)?;

        Ok(Structure { size, world_origin, palette, primary, secondary })
    }

    fn compound<'a>(parent: &'a NbtCompound, name: &'static str) -> Result<&'a NbtCompound, StructureError> {
        match parent.get(name) {
            Some(NbtTag::Compound(compound)) => Ok(compound),
            _ => Err(StructureError::MissingField(name)),
        }
    }

    fn list<'a>(parent: &'a NbtCompound, name: &'static str) -> Result<&'a [NbtTag], StructureError> {
        match parent.get(name) {
            Some(NbtTag::List(items)) => Ok(items),
            _ => Err(StructureError::MissingField(name)),
        }
    }

    fn int_triple(items: &[i32]) -> Option<[i32; 3]> {
        <[i32; 3]>::try_from(items).ok()
    }

    // A decoded Int list or Int Array: exporters write either.
    fn ints<'a>(parent: &'a NbtCompound, name: &'static str) -> Result<&'a [i32], StructureError> {
        match parent.get(name) {
            Some(NbtTag::IntList(items) | NbtTag::IntArray(items)) => Ok(items),
            _ => Err(StructureError::MissingField(name)),
        }
    }

    fn read_size(root: &NbtCompound) -> Result<StructureSize, StructureError> {
        let [x, y, z] = Self::int_triple(Self::ints(root, "size")?).ok_or(StructureError::BadSize)?;
        let axis = |value: i32| u32::try_from(value).map_err(|_| StructureError::BadSize);
        Ok(StructureSize::new(axis(x)?, axis(y)?, axis(z)?))
    }

    fn read_origin(root: &NbtCompound) -> Option<[i32; 3]> {
        Self::ints(root, "structure_world_origin").ok().and_then(Self::int_triple)
    }

    fn read_palette(structure: &NbtCompound) -> Result<Vec<BlockState>, StructureError> {
        let palette = Self::compound(structure, "palette")?;
        let default = Self::compound(palette, "default")?;
        Self::list(default, "block_palette")?.iter().map(Self::read_block_state).collect()
    }

    fn read_block_state(tag: &NbtTag) -> Result<BlockState, StructureError> {
        let NbtTag::Compound(entry) = tag else {
            return Err(StructureError::MissingField("block_palette"));
        };
        let Some(NbtTag::String(name)) = entry.get("name") else {
            return Err(StructureError::MissingField("name"));
        };
        let states = match entry.get("states") {
            Some(NbtTag::Compound(states)) => Self::read_states(states),
            None => BlockStates::default(),
            Some(_) => return Err(StructureError::MissingField("states")),
        };
        Ok(BlockState::new(name.clone(), states))
    }

    // Keeps byte, short, int and string states. Other tag types are not block states.
    fn read_states(states: &NbtCompound) -> BlockStates {
        let pairs = states
            .iter()
            .filter_map(|(key, tag)| {
                let value = match tag {
                    NbtTag::Byte(0) => StateValue::Bool(false),
                    NbtTag::Byte(1) => StateValue::Bool(true),
                    NbtTag::Byte(v) => StateValue::Int(i32::from(*v)),
                    NbtTag::Short(v) => StateValue::Int(i32::from(*v)),
                    NbtTag::Int(v) => StateValue::Int(*v),
                    NbtTag::String(v) => StateValue::Text(v.clone()),
                    _ => return None,
                };
                Some((key.to_string(), value))
            })
            .collect();
        BlockStates::new(pairs)
    }

    // One index layer, an Int list or an Int Array, as little-endian i32 bytes. A long layer is
    // borrowed from the file bytes where the reader left it; a short decoded one is written back
    // into a small buffer.
    fn layer_bytes<'a>(bytes: &'a [u8], tag: &NbtTag, volume: u64) -> Result<Cow<'a, [u8]>, StructureError> {
        let raw = match tag {
            NbtTag::IntList(items) | NbtTag::IntArray(items) => {
                Cow::Owned(items.iter().flat_map(|item| item.to_le_bytes()).collect())
            }
            NbtTag::IntListSpan { offset, len } | NbtTag::IntArraySpan { offset, len } => {
                let end = (*len as usize).checked_mul(4).and_then(|size| offset.checked_add(size));
                let raw = end.and_then(|end| bytes.get(*offset..end));
                Cow::Borrowed(raw.ok_or(StructureError::MissingField("block_indices"))?)
            }
            _ => return Err(StructureError::MissingField("block_indices")),
        };
        let found = (raw.len() / 4) as u64;
        if found != volume {
            return Err(StructureError::IndexCountMismatch { expected: volume, found });
        }
        Ok(raw)
    }

    fn values(raw: &[u8]) -> impl Iterator<Item = i32> + '_ {
        raw.as_chunks::<4>().0.iter().map(|chunk| i32::from_le_bytes(*chunk))
    }

    // -1 is no block; anything else must be a palette position, which is below u16::MAX.
    fn palette_index(index: i32, palette_len: usize) -> Result<u16, StructureError> {
        if index == -1 {
            return Ok(BlockLayer::NONE);
        }
        match usize::try_from(index) {
            Ok(position) if position < palette_len => Ok(position as u16),
            _ => Err(StructureError::PaletteIndexOutOfRange { index }),
        }
    }

    // One u16 per cell, read straight from the i32 bytes.
    fn dense_layer(raw: &[u8], palette_len: usize, stopped: &impl Fn() -> bool) -> Result<BlockLayer, StructureError> {
        let mut cells = Vec::with_capacity(raw.len() / 4);
        for (cell, index) in Self::values(raw).enumerate() {
            Self::check_stop(cell, stopped)?;
            cells.push(Self::palette_index(index, palette_len)?);
        }
        Ok(BlockLayer::new(cells))
    }

    // Keeps the cells that hold a block other than air or structure void. The first pass checks
    // every index and counts the kept cells, so the second pass allocates the exact size.
    fn sparse_layer(
        raw: &[u8],
        palette: &[BlockState],
        stopped: &impl Fn() -> bool,
    ) -> Result<SparseLayer, StructureError> {
        let kept: Vec<bool> = palette.iter().map(|block| !block.is_empty_block()).collect();
        let keep = |index: u16| kept.get(usize::from(index)).copied().unwrap_or(false);
        let mut count = 0usize;
        for (cell, index) in Self::values(raw).enumerate() {
            Self::check_stop(cell, stopped)?;
            if keep(Self::palette_index(index, palette.len())?) {
                count += 1;
            }
        }
        let mut cells = Vec::with_capacity(count);
        let mut blocks = Vec::with_capacity(count);
        for (cell, index) in Self::values(raw).enumerate() {
            let index = Self::palette_index(index, palette.len())?;
            if keep(index) {
                // The volume limit keeps every cell position inside u32.
                cells.push(cell as u32);
                blocks.push(index);
            }
        }
        Ok(SparseLayer::new(cells, blocks))
    }

    fn check_stop(cell: usize, stopped: &impl Fn() -> bool) -> Result<(), StructureError> {
        if cell.is_multiple_of(Self::STOP_EVERY) && stopped() { Err(StructureError::Stopped) } else { Ok(()) }
    }
}
