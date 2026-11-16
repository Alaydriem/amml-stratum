use stratum_core::services::{NbtWriter, SkullPoser};
use stratum_core::structs::marker::Rotation;
use stratum_core::structs::nbt::NbtTag;
use stratum_core::structs::structure::{BlockState, BlockStates, StateValue};

struct Skulls;

impl Skulls {
    const ROTATIONS: [Rotation; 4] = [Rotation::R0, Rotation::R90, Rotation::R180, Rotation::R270];

    fn block(facing: Option<i32>) -> BlockState {
        let states = facing.map(|value| vec![("facing_direction".to_string(), StateValue::Int(value))]);
        BlockState::new("minecraft:skeleton_skull".to_string(), BlockStates::new(states.unwrap_or_default()))
    }

    fn nbt(tag: (&str, NbtTag)) -> Vec<u8> {
        NbtWriter::compound("", &[("id", NbtTag::String("Skull".to_string())), tag]).unwrap()
    }

    fn degrees(block: &BlockState, nbt: &[u8]) -> Vec<f32> {
        Self::ROTATIONS.iter().map(|rotation| SkullPoser::pose(block, Some(nbt), *rotation).degrees).collect()
    }

    fn faces(facing: i32) -> Vec<u8> {
        let block = Self::block(Some(facing));
        Self::ROTATIONS.iter().map(|rotation| SkullPoser::pose(&block, None, *rotation).face).collect()
    }
}

#[test]
fn a_skull_rotation_turns_with_the_ghost() {
    let floor = Skulls::block(Some(1));

    let float = Skulls::nbt(("Rotation", NbtTag::Float(45.0)));
    assert_eq!(Skulls::degrees(&floor, &float), [45.0, 135.0, 225.0, 315.0]);

    // `Rot` is in steps of 22.5 degrees and wraps past 360
    let step = Skulls::nbt(("Rot", NbtTag::Byte(12)));
    assert_eq!(Skulls::degrees(&floor, &step), [270.0, 0.0, 90.0, 180.0]);

    let negative = Skulls::nbt(("Rotation", NbtTag::Float(-90.0)));
    assert_eq!(Skulls::degrees(&floor, &negative), [270.0, 0.0, 90.0, 180.0]);
}

#[test]
fn a_wall_skull_face_turns_with_the_ghost() {
    assert_eq!(Skulls::faces(2), [2, 5, 3, 4]);
    assert_eq!(Skulls::faces(5), [5, 3, 4, 2]);
    assert_eq!(Skulls::faces(3), [3, 4, 2, 5]);
    assert_eq!(Skulls::faces(4), [4, 2, 5, 3]);
    assert_eq!(Skulls::faces(0), [0, 0, 0, 0]);
    assert_eq!(Skulls::faces(1), [1, 1, 1, 1]);
}

#[test]
fn a_skull_without_rotation_nbt_faces_zero() {
    let floor = Skulls::block(Some(1));
    let unrelated = Skulls::nbt(("SkullType", NbtTag::Byte(0)));

    assert_eq!(SkullPoser::pose(&floor, None, Rotation::R0).degrees, 0.0);
    assert_eq!(SkullPoser::pose(&floor, Some(&unrelated), Rotation::R0).degrees, 0.0);
    assert_eq!(SkullPoser::pose(&floor, Some(b"not nbt"), Rotation::R0).degrees, 0.0);
    // a skull with no facing state stands on the floor
    assert_eq!(SkullPoser::pose(&Skulls::block(None), None, Rotation::R0).face, 1);
}

#[test]
fn a_wall_skull_has_no_turn() {
    let nbt = Skulls::nbt(("Rotation", NbtTag::Float(45.0)));
    for facing in [0, 2, 3, 4, 5] {
        assert_eq!(Skulls::degrees(&Skulls::block(Some(facing)), &nbt), [0.0; 4], "facing {facing}");
    }
}

#[test]
fn rot_is_read_before_rotation() {
    let both = NbtWriter::compound("", &[("Rotation", NbtTag::Float(45.0)), ("Rot", NbtTag::Byte(4))]).unwrap();

    assert_eq!(Skulls::degrees(&Skulls::block(Some(1)), &both)[0], 90.0);
}

#[test]
fn a_rotation_that_is_not_finite_faces_zero() {
    let floor = Skulls::block(Some(1));
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let float = Skulls::nbt(("Rotation", NbtTag::Float(value)));
        assert_eq!(Skulls::degrees(&floor, &float), [0.0, 90.0, 180.0, 270.0], "{value}");
    }
    let double = Skulls::nbt(("Rotation", NbtTag::Double(f64::NAN)));
    assert_eq!(Skulls::degrees(&floor, &double), [0.0, 90.0, 180.0, 270.0]);
    let huge = Skulls::nbt(("Rotation", NbtTag::Double(1e300)));
    assert_eq!(Skulls::degrees(&floor, &huge), [0.0, 90.0, 180.0, 270.0], "a double past f32 is infinite");
}
