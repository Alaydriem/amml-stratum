use crate::errors::NbtError;
use crate::structs::nbt::{NbtCompound, NbtLimits, NbtTag};

const TAG_END: u8 = 0;
const TAG_BYTE: u8 = 1;
const TAG_SHORT: u8 = 2;
const TAG_INT: u8 = 3;
const TAG_LONG: u8 = 4;
const TAG_FLOAT: u8 = 5;
const TAG_DOUBLE: u8 = 6;
const TAG_BYTE_ARRAY: u8 = 7;
const TAG_STRING: u8 = 8;
const TAG_LIST: u8 = 9;
const TAG_COMPOUND: u8 = 10;
const TAG_INT_ARRAY: u8 = 11;
const TAG_LONG_ARRAY: u8 = 12;

// Reads little-endian (Bedrock) NBT. Every length is checked against the
// bytes left before memory is reserved for it.
pub struct NbtReader<'a> {
    bytes: &'a [u8],
    pos: usize,
    limits: NbtLimits,
    charged: u64,
}

impl<'a> NbtReader<'a> {
    pub fn read(bytes: &'a [u8], limits: NbtLimits) -> Result<(String, NbtCompound), NbtError> {
        let mut reader = Self { bytes, pos: 0, limits, charged: 0 };
        reader.read_root()
    }

    fn read_root(&mut self) -> Result<(String, NbtCompound), NbtError> {
        if self.read_u8()? != TAG_COMPOUND {
            return Err(NbtError::RootNotCompound);
        }
        let name = self.read_string()?;
        let compound = self.read_compound(1)?;
        Ok((name, compound))
    }

    fn remaining(&self) -> usize {
        self.bytes.len() - self.pos
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], NbtError> {
        if len > self.remaining() {
            return Err(NbtError::Truncated { offset: self.pos });
        }
        let slice = &self.bytes[self.pos..self.pos + len];
        self.pos += len;
        Ok(slice)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], NbtError> {
        let mut out = [0u8; N];
        out.copy_from_slice(self.take(N)?);
        Ok(out)
    }

    fn read_u8(&mut self) -> Result<u8, NbtError> {
        Ok(self.array::<1>()?[0])
    }

    fn read_u16(&mut self) -> Result<u16, NbtError> {
        Ok(u16::from_le_bytes(self.array()?))
    }

    fn read_u32(&mut self) -> Result<u32, NbtError> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    fn charge(&mut self, bytes: u64) -> Result<(), NbtError> {
        self.charged = self.charged.saturating_add(bytes);
        if self.charged > self.limits.max_total_bytes {
            return Err(NbtError::TooLarge);
        }
        Ok(())
    }

    fn read_string(&mut self) -> Result<String, NbtError> {
        let len = usize::from(self.read_u16()?);
        let offset = self.pos;
        let raw = self.take(len)?;
        self.charge(len as u64)?;
        std::str::from_utf8(raw).map(str::to_owned).map_err(|_| NbtError::BadUtf8 { offset })
    }

    fn check_depth(&self, depth: u16) -> Result<(), NbtError> {
        if depth > self.limits.max_depth {
            return Err(NbtError::TooDeep);
        }
        Ok(())
    }

    // Reads an item count, then proves the input holds `count * item_size`
    // bytes and that the decoded form fits the limits.
    fn read_count(&mut self, min_item_size: usize, decoded_item_size: usize) -> Result<usize, NbtError> {
        let len = self.read_u32()?;
        if len > self.limits.max_list_len {
            return Err(NbtError::ListTooLong { len });
        }
        let count = len as usize;
        if count.saturating_mul(min_item_size) > self.remaining() {
            return Err(NbtError::Truncated { offset: self.pos });
        }
        self.charge((count as u64).saturating_mul(decoded_item_size as u64))?;
        Ok(count)
    }

    fn min_item_size(id: u8) -> usize {
        match id {
            TAG_BYTE | TAG_COMPOUND => 1,
            TAG_SHORT | TAG_STRING => 2,
            TAG_INT | TAG_FLOAT | TAG_BYTE_ARRAY | TAG_INT_ARRAY | TAG_LONG_ARRAY => 4,
            TAG_LIST => 5,
            _ => 8,
        }
    }

    fn read_compound(&mut self, depth: u16) -> Result<NbtCompound, NbtError> {
        self.check_depth(depth)?;
        let mut entries = Vec::new();
        loop {
            let id = self.read_u8()?;
            if id == TAG_END {
                break;
            }
            if id > TAG_LONG_ARRAY {
                return Err(NbtError::UnknownTag { id, offset: self.pos - 1 });
            }
            let name = self.read_string()?;
            self.reserve_entry(&mut entries)?;
            let tag = self.read_payload(id, depth)?;
            entries.push((name, tag));
        }
        Ok(NbtCompound::new(entries))
    }

    // Grows the entry vector by doubling and charges the new capacity, so
    // growth slack is counted before it is allocated.
    fn reserve_entry(&mut self, entries: &mut Vec<(String, NbtTag)>) -> Result<(), NbtError> {
        if entries.len() < entries.capacity() {
            return Ok(());
        }
        let capacity = entries.capacity();
        let target = (capacity * 2).max(4);
        self.charge(((target - capacity) * std::mem::size_of::<(String, NbtTag)>()) as u64)?;
        entries.reserve_exact(target - capacity);
        Ok(())
    }

    fn read_list(&mut self, depth: u16) -> Result<NbtTag, NbtError> {
        self.check_depth(depth)?;
        let id_offset = self.pos;
        let id = self.read_u8()?;
        if id > TAG_LONG_ARRAY {
            return Err(NbtError::UnknownTag { id, offset: id_offset });
        }
        if id == TAG_INT {
            return self.read_ints(false);
        }
        let count = self.read_count(Self::min_item_size(id), std::mem::size_of::<NbtTag>())?;
        if count > 0 && id == TAG_END {
            return Err(NbtError::UnknownTag { id, offset: id_offset });
        }
        let mut items = Vec::with_capacity(count);
        for _ in 0..count {
            items.push(self.read_payload(id, depth)?);
        }
        Ok(NbtTag::List(items))
    }

    // The payload of an Int list (`array` false) or an Int Array (`array` true): the same bytes on the
    // wire. A short one decodes to `IntList` / `IntArray`. A long one is only checked and skipped, and
    // becomes a span over the input, so no full i32 copy of it is ever made.
    fn read_ints(&mut self, array: bool) -> Result<NbtTag, NbtError> {
        let len_offset = self.pos;
        let len = self.read_u32()?;
        self.pos = len_offset;
        if len <= self.limits.max_decoded_int_list {
            let count = self.read_count(4, 4)?;
            let raw = self.take(count * 4)?;
            let items = raw.as_chunks::<4>().0.iter().map(|c| i32::from_le_bytes(*c)).collect();
            return Ok(if array { NbtTag::IntArray(items) } else { NbtTag::IntList(items) });
        }
        let count = self.read_count(4, 2)?;
        let offset = self.pos;
        self.take(count * 4)?;
        Ok(if array { NbtTag::IntArraySpan { offset, len } } else { NbtTag::IntListSpan { offset, len } })
    }

    fn read_payload(&mut self, id: u8, depth: u16) -> Result<NbtTag, NbtError> {
        match id {
            TAG_BYTE => Ok(NbtTag::Byte(i8::from_le_bytes(self.array()?))),
            TAG_SHORT => Ok(NbtTag::Short(i16::from_le_bytes(self.array()?))),
            TAG_INT => Ok(NbtTag::Int(i32::from_le_bytes(self.array()?))),
            TAG_LONG => Ok(NbtTag::Long(i64::from_le_bytes(self.array()?))),
            TAG_FLOAT => Ok(NbtTag::Float(f32::from_le_bytes(self.array()?))),
            TAG_DOUBLE => Ok(NbtTag::Double(f64::from_le_bytes(self.array()?))),
            TAG_BYTE_ARRAY => {
                let count = self.read_count(1, 1)?;
                let raw = self.take(count)?;
                Ok(NbtTag::ByteArray(raw.iter().map(|b| *b as i8).collect()))
            }
            TAG_STRING => Ok(NbtTag::String(self.read_string()?)),
            TAG_LIST => self.read_list(depth + 1),
            TAG_COMPOUND => Ok(NbtTag::Compound(self.read_compound(depth + 1)?)),
            TAG_INT_ARRAY => self.read_ints(true),
            TAG_LONG_ARRAY => {
                let count = self.read_count(8, 8)?;
                let raw = self.take(count * 8)?;
                Ok(NbtTag::LongArray(raw.as_chunks::<8>().0.iter().map(|c| i64::from_le_bytes(*c)).collect()))
            }
            _ => Err(NbtError::UnknownTag { id, offset: self.pos.saturating_sub(1) }),
        }
    }
}
