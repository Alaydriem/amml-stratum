use stratum_core::errors::StructureError;
use stratum_core::services::StructureParser;
use stratum_core::structs::nbt::NbtTag;
use stratum_core::structs::structure::{BlockLayer, StateValue};

use crate::support::{NbtWriter, StructureFixture};

#[test]
fn parses_size_palette_and_indices() {
    let bytes =
        StructureFixture::new([2, 3, 4]).block(1, 2, 3, "minecraft:stone").block(0, 0, 0, "minecraft:dirt").bytes();
    let structure = StructureParser::parse(&bytes).unwrap();

    assert_eq!((structure.size.x, structure.size.y, structure.size.z), (2, 3, 4));
    assert_eq!(structure.size.volume(), 24);
    assert_eq!(structure.palette.len(), 2);
    assert_eq!(structure.palette[0].name, "minecraft:stone");
    assert_eq!(structure.primary.len(), 24);
    assert_eq!(structure.secondary.len(), 0);
    // (1 * 3 + 2) * 4 + 3 = 23
    assert_eq!(structure.primary.get(23), Some(0));
    assert_eq!(structure.primary.get(0), Some(1));
}

#[test]
fn index_order_matches_bedrock() {
    let mut fixture = StructureFixture::new([2, 2, 2]);
    for x in 0..2usize {
        for y in 0..2usize {
            for z in 0..2usize {
                fixture = fixture.block(x, y, z, &format!("test:b{x}{y}{z}"));
            }
        }
    }
    let structure = StructureParser::parse(&fixture.bytes()).unwrap();

    for x in 0..2u32 {
        for y in 0..2u32 {
            for z in 0..2u32 {
                let block = structure.block_at(x, y, z).unwrap();
                assert_eq!(block.name, format!("test:b{x}{y}{z}"));
            }
        }
    }
    assert_eq!(structure.index_of(1, 0, 0), 4);
    assert_eq!(structure.index_of(0, 1, 0), 2);
    assert_eq!(structure.index_of(0, 0, 1), 1);
    assert!(structure.block_at(2, 0, 0).is_none());
}

#[test]
fn void_cells_read_as_no_block() {
    let bytes = StructureFixture::new([2, 1, 1]).block(0, 0, 0, "minecraft:stone").bytes();
    let structure = StructureParser::parse(&bytes).unwrap();

    assert!(structure.block_at(0, 0, 0).is_some());
    assert!(structure.block_at(1, 0, 0).is_none());
    // -1 in the file is stored as the no-block value.
    assert_eq!(structure.primary.as_slice(), [0, BlockLayer::NONE]);
    assert_eq!(structure.primary.get(1), None);
    assert_eq!(structure.secondary.get(0), None);
}

#[test]
fn waterlog_layer_is_sparse() {
    let small = StructureFixture::new([4, 4, 4])
        .block(0, 0, 0, "minecraft:oak_stairs")
        .waterlog(0, 0, 0, "minecraft:water")
        .waterlog(3, 3, 3, "minecraft:water")
        .bytes();
    let big = StructureFixture::new([64, 64, 64])
        .block(0, 0, 0, "minecraft:oak_stairs")
        .waterlog(0, 0, 0, "minecraft:water")
        .waterlog(63, 63, 63, "minecraft:water")
        .bytes();
    let small = StructureParser::parse(&small).unwrap();
    let big = StructureParser::parse(&big).unwrap();

    assert_eq!(small.secondary.len(), 2);
    assert_eq!(big.secondary.len(), 2);
    assert_eq!(small.secondary.get(0), Some(1));
    assert_eq!(small.secondary.get(small.index_of(3, 3, 3)), Some(2));
    assert_eq!(small.secondary.get(1), None);
    assert_eq!(big.secondary.get(big.index_of(63, 63, 63)), Some(2));
    // 4,096 cells against 262,144: the layer costs the same.
    assert_eq!(small.secondary.heap_bytes(), big.secondary.heap_bytes());
    assert!(big.secondary.heap_bytes() <= 2 * 8);
    let pairs: Vec<(u32, u16)> = big.secondary.iter().collect();
    assert_eq!(pairs, [(0, 1), (262_143, 2)]);
}

#[test]
fn waterlog_layer_drops_air_and_out_of_range_is_refused() {
    let bytes = StructureFixture::new([2, 1, 1])
        .block(0, 0, 0, "minecraft:stone")
        .block(1, 0, 0, "minecraft:air")
        .raw_indices(vec![0, 1], vec![1, 0])
        .bytes();
    let structure = StructureParser::parse(&bytes).unwrap();
    let pairs: Vec<(u32, u16)> = structure.secondary.iter().collect();
    assert_eq!(pairs, [(1, 0)]);

    let bytes = StructureFixture::new([2, 1, 1])
        .block(0, 0, 0, "minecraft:stone")
        .raw_indices(vec![0, -1], vec![-1, 3])
        .bytes();
    assert_eq!(StructureParser::parse(&bytes).unwrap_err(), StructureError::PaletteIndexOutOfRange { index: 3 });
}

#[test]
fn palette_of_65535_entries_is_refused() {
    let largest = StructureFixture::new([1, 1, 1]).unused_entries(65_533).block(0, 0, 0, "minecraft:stone").bytes();
    let structure = StructureParser::parse(&largest).unwrap();
    assert_eq!(structure.palette.len(), 65_534);
    assert_eq!(structure.block_at(0, 0, 0).map(|block| block.name.as_str()), Some("minecraft:stone"));
    assert_eq!(structure.primary.get(0), Some(65_533));

    let bytes = StructureFixture::new([1, 1, 1]).unused_entries(65_534).block(0, 0, 0, "minecraft:stone").bytes();
    assert_eq!(StructureParser::parse(&bytes).unwrap_err(), StructureError::PaletteTooLarge { entries: 65_535 });
}

#[test]
fn reads_world_origin_when_present() {
    let bytes = StructureFixture::new([1, 1, 1]).origin([10, -64, -30]).bytes();
    let structure = StructureParser::parse(&bytes).unwrap();

    assert_eq!(structure.world_origin, Some([10, -64, -30]));
}

#[test]
fn world_origin_is_none_when_absent() {
    let structure = StructureParser::parse(&StructureFixture::new([1, 1, 1]).bytes()).unwrap();

    assert_eq!(structure.world_origin, None);
}

#[test]
fn block_count_skips_air_and_structure_void() {
    let bytes = StructureFixture::new([5, 1, 1])
        .block(0, 0, 0, "minecraft:stone")
        .block(1, 0, 0, "minecraft:air")
        .block(2, 0, 0, "minecraft:structure_void")
        .block(3, 0, 0, "minecraft:dirt")
        .bytes();
    let structure = StructureParser::parse(&bytes).unwrap();

    assert_eq!(structure.block_count(), 2);
}

#[test]
fn rejects_index_count_that_does_not_match_size() {
    let bytes = StructureFixture::new([2, 2, 2]).raw_indices(vec![-1; 7], vec![-1; 8]).bytes();

    assert_eq!(
        StructureParser::parse(&bytes).unwrap_err(),
        StructureError::IndexCountMismatch { expected: 8, found: 7 }
    );
}

#[test]
fn rejects_palette_index_out_of_range() {
    let bytes = StructureFixture::new([2, 1, 1])
        .block(0, 0, 0, "minecraft:stone")
        .raw_indices(vec![0, 5], vec![-1, -1])
        .bytes();

    assert_eq!(StructureParser::parse(&bytes).unwrap_err(), StructureError::PaletteIndexOutOfRange { index: 5 });
}

#[test]
fn refuses_one_cell_more() {
    // 256 * 384 * 256 + 1 cells, with no index data behind it.
    let bytes = StructureFixture::new([25_165_825, 1, 1]).raw_indices(Vec::new(), Vec::new()).bytes();
    assert_eq!(StructureParser::parse(&bytes).unwrap_err(), StructureError::TooLarge { volume: 25_165_825 });

    let bytes = StructureFixture::new([256, 384, 257]).raw_indices(Vec::new(), Vec::new()).bytes();
    assert_eq!(StructureParser::parse(&bytes).unwrap_err(), StructureError::TooLarge { volume: 25_264_128 });
}

#[test]
fn byte_states_0_and_1_become_bools() {
    let bytes = StructureFixture::new([1, 1, 1])
        .block(0, 0, 0, "minecraft:lever")
        .state("open_bit", NbtTag::Byte(1))
        .state("powered", NbtTag::Byte(0))
        .state("power", NbtTag::Byte(7))
        .state("facing", NbtTag::String("north".to_string()))
        .state("age", NbtTag::Int(3))
        .bytes();
    let structure = StructureParser::parse(&bytes).unwrap();
    let states = &structure.block_at(0, 0, 0).unwrap().states;

    assert_eq!(states.get("open_bit"), Some(&StateValue::Bool(true)));
    assert_eq!(states.get("powered"), Some(&StateValue::Bool(false)));
    assert_eq!(states.get("power"), Some(&StateValue::Int(7)));
    assert_eq!(states.get("facing"), Some(&StateValue::Text("north".to_string())));
    assert_eq!(states.get("age"), Some(&StateValue::Int(3)));
    let order: Vec<&str> = states.iter().map(|(key, _)| key).collect();
    assert_eq!(order, ["open_bit", "powered", "power", "facing", "age"]);
}

#[test]
fn rejects_volume_that_overflows_u64() {
    // 2^22 * 2^21 * 2^21 is exactly 2^64.
    let bytes = StructureFixture::new([1 << 22, 1 << 21, 1 << 21]).raw_indices(Vec::new(), Vec::new()).bytes();
    assert!(matches!(StructureParser::parse(&bytes).unwrap_err(), StructureError::TooLarge { .. }));

    let bytes = StructureFixture::new([i32::MAX; 3]).raw_indices(Vec::new(), Vec::new()).bytes();
    assert!(matches!(StructureParser::parse(&bytes).unwrap_err(), StructureError::TooLarge { .. }));
}

#[test]
fn accepts_256_x_384_x_256() {
    let bytes = StructureFixture::full([256, 384, 256]).bytes();
    let structure = StructureParser::parse(&bytes).unwrap();

    assert_eq!(structure.size.volume(), 25_165_824);
    assert_eq!(structure.primary.len(), 25_165_824);
    assert_eq!(structure.secondary.len(), 0);
    assert_eq!(structure.block_count(), 25_165_824);
    assert_eq!(structure.block_at(255, 383, 255).map(|block| block.name.as_str()), Some("minecraft:stone"));
    assert!(structure.block_at(256, 0, 0).is_none());
}

#[test]
fn accepts_256_x_384_x_256_fully_waterlogged() {
    let bytes = StructureFixture::full_waterlogged([256, 384, 256]).bytes();
    let structure = StructureParser::parse(&bytes).unwrap();

    assert_eq!(structure.primary.len(), 25_165_824);
    assert_eq!(structure.secondary.len(), 25_165_824);
    assert_eq!(structure.secondary.get(25_165_823), Some(1));
    assert_eq!(structure.block_count(), 25_165_824);
}

#[test]
fn indices_below_minus_one_or_at_the_palette_length_are_refused() {
    // 4 cells decode as an Int list; 5,000 cells stay in the input and are read in place.
    for cells in [4usize, 5_000] {
        for bad in [-2, 1] {
            let mut primary = vec![-1; cells];
            primary[cells - 1] = bad;
            let bytes = StructureFixture::new([cells as i32, 1, 1])
                .block(0, 0, 0, "minecraft:stone")
                .raw_indices(primary, vec![-1; cells])
                .bytes();
            assert_eq!(
                StructureParser::parse(&bytes).unwrap_err(),
                StructureError::PaletteIndexOutOfRange { index: bad },
                "primary, {cells} cells"
            );

            let mut secondary = vec![-1; cells];
            secondary[cells - 1] = bad;
            let bytes = StructureFixture::new([cells as i32, 1, 1])
                .block(0, 0, 0, "minecraft:stone")
                .raw_indices(vec![-1; cells], secondary)
                .bytes();
            assert_eq!(
                StructureParser::parse(&bytes).unwrap_err(),
                StructureError::PaletteIndexOutOfRange { index: bad },
                "secondary, {cells} cells"
            );
        }
    }
}

#[test]
fn large_layers_read_in_place_match_small_ones() {
    let bytes = StructureFixture::new([50, 10, 10])
        .block(0, 0, 0, "minecraft:stone")
        .block(49, 9, 9, "minecraft:dirt")
        .waterlog(49, 9, 9, "minecraft:water")
        .bytes();
    let structure = StructureParser::parse(&bytes).unwrap();

    assert_eq!(structure.block_at(0, 0, 0).map(|block| block.name.as_str()), Some("minecraft:stone"));
    assert_eq!(structure.block_at(49, 9, 9).map(|block| block.name.as_str()), Some("minecraft:dirt"));
    assert!(structure.block_at(1, 0, 0).is_none());
    assert_eq!(structure.block_count(), 2);
    let pairs: Vec<(u32, u16)> = structure.secondary.iter().collect();
    assert_eq!(pairs, [(4_999, 2)]);
}

// Laid out like the files some exporters write: block_indices is a list of two Int Arrays.
#[test]
fn layers_written_as_int_arrays_parse_like_int_lists() {
    let bytes = StructureFixture::new([3, 2, 2])
        .block(0, 0, 0, "minecraft:oak_log")
        .block(2, 1, 1, "minecraft:oak_leaves")
        .block(1, 0, 0, "minecraft:air")
        .waterlog(2, 1, 1, "minecraft:water")
        .origin([100, 64, -200])
        .int_array_layers()
        .bytes();
    let structure = StructureParser::parse(&bytes).unwrap();

    assert_eq!((structure.size.x, structure.size.y, structure.size.z), (3, 2, 2));
    assert_eq!(structure.world_origin, Some([100, 64, -200]));
    assert_eq!(structure.palette.len(), 4);
    assert_eq!(structure.block_count(), 2);
    assert_eq!(structure.block_at(2, 1, 1).map(|block| block.name.as_str()), Some("minecraft:oak_leaves"));
    let pairs: Vec<(u32, u16)> = structure.secondary.iter().collect();
    assert_eq!(pairs, [(11, 3)]);
}

// Header and layers are read independently: Int Array header with Int list layers, and all Int Arrays.
#[test]
fn size_and_origin_written_as_int_arrays_are_read() {
    for layers_as_arrays in [false, true] {
        let fixture =
            StructureFixture::new([2, 1, 3]).block(1, 0, 2, "minecraft:stone").origin([-5, 0, 7]).int_array_header();
        let fixture = if layers_as_arrays { fixture.int_array_layers() } else { fixture };
        let structure = StructureParser::parse(&fixture.bytes()).unwrap();

        assert_eq!((structure.size.x, structure.size.y, structure.size.z), (2, 1, 3), "arrays {layers_as_arrays}");
        assert_eq!(structure.world_origin, Some([-5, 0, 7]));
        assert_eq!(structure.block_count(), 1);
    }
}

#[test]
fn a_size_array_that_is_not_three_ints_is_refused() {
    let bytes = NbtWriter::compound("", &[("size", NbtTag::IntArray(vec![2, 2]))]).expect("fixture NBT is writable");

    assert_eq!(StructureParser::parse(&bytes).unwrap_err(), StructureError::BadSize);
}

#[test]
fn an_int_array_layer_of_the_wrong_length_is_refused() {
    let bytes = StructureFixture::new([2, 2, 2]).raw_indices(vec![-1; 8], vec![-1; 9]).int_array_layers().bytes();

    assert_eq!(
        StructureParser::parse(&bytes).unwrap_err(),
        StructureError::IndexCountMismatch { expected: 8, found: 9 }
    );
}

#[test]
fn large_int_array_layers_read_in_place_match_small_ones() {
    let bytes = StructureFixture::new([50, 10, 10])
        .block(0, 0, 0, "minecraft:stone")
        .block(49, 9, 9, "minecraft:dirt")
        .waterlog(49, 9, 9, "minecraft:water")
        .int_array_layers()
        .bytes();
    let structure = StructureParser::parse(&bytes).unwrap();

    assert_eq!(structure.block_at(0, 0, 0).map(|block| block.name.as_str()), Some("minecraft:stone"));
    assert_eq!(structure.block_at(49, 9, 9).map(|block| block.name.as_str()), Some("minecraft:dirt"));
    assert_eq!(structure.block_count(), 2);
    let pairs: Vec<(u32, u16)> = structure.secondary.iter().collect();
    assert_eq!(pairs, [(4_999, 2)]);
}

// Decoded at 4 bytes per item, two full Int Array layers pass the byte limit; as spans they fit.
#[test]
fn accepts_256_x_384_x_256_fully_waterlogged_as_int_arrays() {
    let bytes = StructureFixture::full_waterlogged([256, 384, 256]).int_array_layers().bytes();
    let structure = StructureParser::parse(&bytes).unwrap();

    assert_eq!(structure.primary.len(), 25_165_824);
    assert_eq!(structure.secondary.len(), 25_165_824);
    assert_eq!(structure.block_count(), 25_165_824);
}

#[test]
fn a_parse_stops_when_asked_and_reads_the_whole_file_when_not() {
    let bytes = StructureFixture::full([64, 64, 64]).bytes();
    let asks = std::cell::Cell::new(0u32);
    let stopped = StructureParser::parse_until(&bytes, || {
        asks.set(asks.get() + 1);
        asks.get() > 2
    });
    // 64^3 cells give four checks per layer, so the third check comes inside the first layer
    assert_eq!(stopped, Err(StructureError::Stopped));
    assert!(StructureParser::parse_until(&bytes, || false).is_ok());
}
