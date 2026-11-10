use super::NbtCompound;

#[derive(Debug, Clone, PartialEq)]
pub enum NbtTag {
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<i8>),
    String(String),
    List(Vec<NbtTag>),
    // A list whose element type is Int, kept as plain integers.
    IntList(Vec<i32>),
    // A list of Int longer than `NbtLimits::max_decoded_int_list`, left undecoded: `len`
    // little-endian i32 values start at `offset` in the bytes given to the reader.
    IntListSpan { offset: usize, len: u32 },
    Compound(NbtCompound),
    IntArray(Vec<i32>),
    // An Int Array longer than `NbtLimits::max_decoded_int_list`, left undecoded like `IntListSpan`.
    IntArraySpan { offset: usize, len: u32 },
    LongArray(Vec<i64>),
}
