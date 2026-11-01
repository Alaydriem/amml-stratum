use stratum_core::structs::nbt::{NbtCompound, NbtTag};

use super::NbtWriter;

// Ready-made NBT inputs shared by reader and parser tests.
pub struct NbtFixtures;

impl NbtFixtures {
    // A root compound named "root" holding one tag of each of the 12 types.
    pub fn every_type() -> Vec<u8> {
        let inner = NbtCompound::new(vec![("k".to_string(), NbtTag::Int(7))]);
        NbtWriter::compound(
            "root",
            &[
                ("byte", NbtTag::Byte(-5)),
                ("short", NbtTag::Short(-1234)),
                ("int", NbtTag::Int(0x0102_0304)),
                ("long", NbtTag::Long(-9_000_000_000)),
                ("float", NbtTag::Float(1.5)),
                ("double", NbtTag::Double(-2.25)),
                ("bytes", NbtTag::ByteArray(vec![1, -2, 3])),
                ("string", NbtTag::String("hello".to_string())),
                ("list", NbtTag::List(vec![NbtTag::Short(1), NbtTag::Short(2)])),
                ("compound", NbtTag::Compound(inner)),
                ("ints", NbtTag::IntArray(vec![10, -20, 30])),
                ("longs", NbtTag::LongArray(vec![i64::MIN, 0, i64::MAX])),
            ],
        )
        .expect("fixture NBT is writable")
    }

    // `depth` compounds nested in each other, innermost empty.
    pub fn nested(depth: usize) -> NbtTag {
        let mut tag = NbtTag::Compound(NbtCompound::default());
        for _ in 1..depth {
            tag = NbtTag::Compound(NbtCompound::new(vec![("c".to_string(), tag)]));
        }
        tag
    }
}
