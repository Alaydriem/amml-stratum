use stratum_core::structs::block::Face;

#[test]
fn the_index_follows_the_game_facing_order() {
    let names = ["down", "up", "north", "south", "west", "east"];
    for (position, face) in Face::ALL.iter().enumerate() {
        assert_eq!(face.index(), position);
        assert_eq!(face.json_key(), names[position]);
    }
}

#[test]
fn the_opposite_face_points_the_other_way() {
    for face in Face::ALL {
        assert_eq!(face.opposite().opposite(), face);
        let (a, b) = (face.normal(), face.opposite().normal());
        assert_eq!([a[0] + b[0], a[1] + b[1], a[2] + b[2]], [0, 0, 0]);
    }
}
