use alaydriem_stratum::ghost::SkullPoseMap;
use amml_api::{Face, SkullPose};
use stratum_core::structs::ghost::SkullPose as CoreSkullPose;

#[test]
fn a_floor_skull_takes_its_turn_in_sixteenths_of_a_full_turn() {
    assert_eq!(SkullPoseMap::pose(CoreSkullPose::new(1, 0.0)), Some(SkullPose { face: Face::Up, turn: 0 }));
    assert_eq!(SkullPoseMap::pose(CoreSkullPose::new(1, 90.0)), Some(SkullPose { face: Face::Up, turn: 4 }));
    assert_eq!(SkullPoseMap::pose(CoreSkullPose::new(1, 337.5)), Some(SkullPose { face: Face::Up, turn: 15 }));
    assert_eq!(SkullPoseMap::pose(CoreSkullPose::new(1, 360.0)), Some(SkullPose { face: Face::Up, turn: 0 }));
}

#[test]
fn a_wall_skull_hangs_from_its_face_and_is_never_turned() {
    let faces = [(2, Face::North), (3, Face::South), (4, Face::West), (5, Face::East)];

    for (facing, face) in faces {
        assert_eq!(SkullPoseMap::pose(CoreSkullPose::new(facing, 90.0)), Some(SkullPose { face, turn: 0 }));
    }
}

#[test]
fn a_facing_that_is_no_skull_face_gives_no_pose() {
    assert_eq!(SkullPoseMap::pose(CoreSkullPose::new(0, 0.0)), None);
    assert_eq!(SkullPoseMap::pose(CoreSkullPose::new(6, 0.0)), None);
}
