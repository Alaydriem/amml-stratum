use stratum_core::services::{BlockEntityTurner, NbtReader, NbtWriter};
use stratum_core::structs::marker::Rotation;
use stratum_core::structs::nbt::{NbtLimits, NbtTag};

struct Nbt;

impl Nbt {
    fn shulker(facing: i8) -> Vec<u8> {
        NbtWriter::compound("", &[("id", NbtTag::String("ShulkerBox".to_string())), ("facing", NbtTag::Byte(facing))])
            .unwrap()
    }

    fn chest(pair_x: i32, pair_z: i32) -> Vec<u8> {
        NbtWriter::compound(
            "",
            &[
                ("id", NbtTag::String("Chest".to_string())),
                ("pairx", NbtTag::Int(pair_x)),
                ("pairlead", NbtTag::Byte(1)),
                ("pairz", NbtTag::Int(pair_z)),
            ],
        )
        .unwrap()
    }

    fn facing(bytes: &[u8]) -> Option<i8> {
        let (_, root) = NbtReader::read(bytes, NbtLimits::STRUCTURE).unwrap();
        match root.get("facing") {
            Some(NbtTag::Byte(facing)) => Some(*facing),
            _ => None,
        }
    }
}

#[test]
fn shulker_facing_turns_clockwise_each_quarter() {
    let rotations = [Rotation::R0, Rotation::R90, Rotation::R180, Rotation::R270];
    let turn = |facing: i8, rotation: Rotation| match BlockEntityTurner::turn(
        &Nbt::shulker(facing),
        rotation,
        [0; 3],
        [0; 3],
        None,
    ) {
        Some(bytes) => Nbt::facing(&bytes).unwrap(),
        None => facing,
    };

    let north: Vec<i8> = rotations.iter().map(|rotation| turn(2, *rotation)).collect();
    assert_eq!(north, [2, 5, 3, 4]);
    for rotation in rotations {
        assert_eq!(turn(0, rotation), 0);
        assert_eq!(turn(1, rotation), 1);
    }
    assert_eq!(BlockEntityTurner::turn(&Nbt::shulker(1), Rotation::R90, [0; 3], [0; 3], None), None);
}

#[test]
fn nbt_without_a_turning_key_is_not_rewritten() {
    let banner = NbtWriter::compound(
        "",
        &[("id", NbtTag::String("Banner".to_string())), ("Base", NbtTag::Int(3)), ("facing", NbtTag::Byte(2))],
    )
    .unwrap();

    assert_eq!(BlockEntityTurner::turn(&banner, Rotation::R90, [0; 3], [0; 3], None), None);
    assert!(!BlockEntityTurner::depends_on_cell(&banner));
    assert_eq!(BlockEntityTurner::chest_pair_offset(&banner, [0; 3], None), None);
    assert_eq!(BlockEntityTurner::turn(b"not nbt", Rotation::R90, [0; 3], [0; 3], None), None);
}

#[test]
fn a_chest_pair_offset_turns_with_the_structure() {
    // The chest is cell (0, 0, 0) of a structure cut at world (10, 64, 5); its pair is one cell east.
    let source = Nbt::chest(11, 5);
    let origin = Some([10, 64, 5]);
    assert!(BlockEntityTurner::depends_on_cell(&source));

    for rotation in [Rotation::R0, Rotation::R90, Rotation::R180, Rotation::R270] {
        // A 2 x 1 x 1 structure: where the cell lands is the turned corner shift.
        let corner = rotation.turn_corner([2, 1, 1]);
        let turned_cell = rotation.turn_offset([0, 0, 0]).map(|value| value as i32);
        let turned_cell = [0, 1, 2].map(|axis| turned_cell[axis] - corner[axis] as i32);

        let turned =
            BlockEntityTurner::turn(&source, rotation, [0; 3], turned_cell, origin).unwrap_or_else(|| source.clone());
        let offset = BlockEntityTurner::chest_pair_offset(&turned, turned_cell, origin).unwrap();

        let expected = rotation.turn_offset([1, 0, 0]).map(|value| value as i32);
        assert_eq!(offset, expected, "{rotation:?}");
    }
}

#[test]
fn a_chest_pair_without_a_world_origin_is_read_as_local() {
    let source = Nbt::chest(3, 2);

    let turned = BlockEntityTurner::turn(&source, Rotation::R180, [2, 0, 2], [0, 0, 0], None).unwrap();

    // The pair was at offset (1, 0, 0) from the cell; a half turn makes it (-1, 0, 0).
    assert_eq!(BlockEntityTurner::chest_pair_offset(&turned, [0, 0, 0], None), Some([-1, 0, 0]));
}

#[test]
fn the_pair_keys_are_removed_and_the_rest_stays() {
    let stripped = BlockEntityTurner::strip_pair(&Nbt::chest(11, 5)).unwrap();

    let (_, root) = NbtReader::read(&stripped, NbtLimits::STRUCTURE).unwrap();
    assert_eq!(root.len(), 1);
    assert!(matches!(root.get("id"), Some(NbtTag::String(id)) if id == "Chest"));
    // two chests of different pairs end up with the same bytes
    assert_eq!(Some(stripped), BlockEntityTurner::strip_pair(&Nbt::chest(40, -3)));
    assert_eq!(BlockEntityTurner::strip_pair(&Nbt::shulker(2)), None);
    assert_eq!(BlockEntityTurner::strip_pair(b"pair but not nbt"), None);
}
