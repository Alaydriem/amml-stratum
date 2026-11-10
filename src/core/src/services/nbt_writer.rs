use crate::errors::NbtWriteError;
use crate::structs::nbt::NbtTag;

// Builds little-endian NBT bytes.
pub struct NbtWriter;

impl NbtWriter {
    // A named root compound holding `entries` in order.
    pub fn compound(name: &str, entries: &[(&str, NbtTag)]) -> Result<Vec<u8>, NbtWriteError> {
        let mut out = vec![10];
        Self::write_string(&mut out, name)?;
        Self::write_entries(&mut out, entries.iter().map(|(n, t)| (*n, t)))?;
        Ok(out)
    }

    // The wire id of a tag.
    fn tag_id(tag: &NbtTag) -> u8 {
        match tag {
            NbtTag::Byte(_) => 1,
            NbtTag::Short(_) => 2,
            NbtTag::Int(_) => 3,
            NbtTag::Long(_) => 4,
            NbtTag::Float(_) => 5,
            NbtTag::Double(_) => 6,
            NbtTag::ByteArray(_) => 7,
            NbtTag::String(_) => 8,
            NbtTag::List(_) | NbtTag::IntList(_) | NbtTag::IntListSpan { .. } => 9,
            NbtTag::Compound(_) => 10,
            NbtTag::IntArray(_) | NbtTag::IntArraySpan { .. } => 11,
            NbtTag::LongArray(_) => 12,
        }
    }

    pub fn write_string(out: &mut Vec<u8>, value: &str) -> Result<(), NbtWriteError> {
        let len = u16::try_from(value.len()).map_err(|_| NbtWriteError::TooLong { len: value.len() })?;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(value.as_bytes());
        Ok(())
    }

    fn write_len(out: &mut Vec<u8>, len: usize) -> Result<(), NbtWriteError> {
        let len = u32::try_from(len).map_err(|_| NbtWriteError::TooLong { len })?;
        out.extend_from_slice(&len.to_le_bytes());
        Ok(())
    }

    // One named entry: tag id, name, payload.
    pub fn write_entry(out: &mut Vec<u8>, name: &str, tag: &NbtTag) -> Result<(), NbtWriteError> {
        out.push(Self::tag_id(tag));
        Self::write_string(out, name)?;
        Self::write_payload(out, tag)
    }

    // The payload of an Int list holding `count` copies of `value`.
    pub fn write_int_list_fill(out: &mut Vec<u8>, value: i32, count: usize) -> Result<(), NbtWriteError> {
        out.push(3);
        Self::write_int_fill(out, value, count)
    }

    // The payload of an Int Array holding `count` copies of `value`, copied from one 64 KiB block
    // so no second buffer the size of the array is made.
    pub fn write_int_fill(out: &mut Vec<u8>, value: i32, count: usize) -> Result<(), NbtWriteError> {
        const BLOCK_ITEMS: usize = 16 * 1024;
        Self::write_len(out, count)?;
        let block = value.to_le_bytes().repeat(BLOCK_ITEMS);
        let mut left = count;
        while left > 0 {
            let items = left.min(BLOCK_ITEMS);
            out.extend_from_slice(&block[..items * 4]);
            left -= items;
        }
        Ok(())
    }

    fn write_entries<'a>(
        out: &mut Vec<u8>,
        entries: impl Iterator<Item = (&'a str, &'a NbtTag)>,
    ) -> Result<(), NbtWriteError> {
        for (name, tag) in entries {
            Self::write_entry(out, name, tag)?;
        }
        out.push(0);
        Ok(())
    }

    fn write_payload(out: &mut Vec<u8>, tag: &NbtTag) -> Result<(), NbtWriteError> {
        match tag {
            NbtTag::Byte(v) => out.push(*v as u8),
            NbtTag::Short(v) => out.extend_from_slice(&v.to_le_bytes()),
            NbtTag::Int(v) => out.extend_from_slice(&v.to_le_bytes()),
            NbtTag::Long(v) => out.extend_from_slice(&v.to_le_bytes()),
            NbtTag::Float(v) => out.extend_from_slice(&v.to_le_bytes()),
            NbtTag::Double(v) => out.extend_from_slice(&v.to_le_bytes()),
            NbtTag::ByteArray(v) => {
                Self::write_len(out, v.len())?;
                out.extend(v.iter().map(|b| *b as u8));
            }
            NbtTag::String(v) => Self::write_string(out, v)?,
            NbtTag::List(items) => {
                let id = items.first().map(Self::tag_id).unwrap_or(0);
                out.push(id);
                Self::write_len(out, items.len())?;
                for item in items {
                    Self::write_payload(out, item)?;
                }
            }
            NbtTag::IntList(items) => {
                out.push(3);
                Self::write_len(out, items.len())?;
                out.extend(items.iter().flat_map(|item| item.to_le_bytes()));
            }
            NbtTag::IntListSpan { .. } | NbtTag::IntArraySpan { .. } => return Err(NbtWriteError::SpanNotWritable),
            NbtTag::Compound(c) => Self::write_entries(out, c.iter())?,
            NbtTag::IntArray(v) => {
                Self::write_len(out, v.len())?;
                for x in v {
                    out.extend_from_slice(&x.to_le_bytes());
                }
            }
            NbtTag::LongArray(v) => {
                Self::write_len(out, v.len())?;
                for x in v {
                    out.extend_from_slice(&x.to_le_bytes());
                }
            }
        }
        Ok(())
    }
}
