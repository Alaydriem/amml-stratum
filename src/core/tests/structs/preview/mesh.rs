use stratum_core::structs::preview::{PreviewMesh, PreviewQuad};
use stratum_core::structs::structure::StructureSize;

// Quads for the mesh tests.
struct Quads;

impl Quads {
    fn at(x: i16) -> PreviewQuad {
        PreviewQuad {
            corners: [[x, 0, 0], [x + 16, 0, 0], [x + 16, 16, 0], [x, 16, 0]],
            uvs: [[0, 0], [16, 0], [16, 16], [0, 16]],
            texture: 0,
            shade: 255,
            normal: [0, 0, -1],
        }
    }
}

#[test]
fn a_push_past_the_bound_is_dropped_and_marks_the_mesh_truncated() {
    let mut mesh = PreviewMesh::new(StructureSize::new(4, 1, 1), Vec::new(), 2);
    mesh.push(Quads::at(0));
    mesh.push(Quads::at(16));
    assert!(!mesh.truncated);
    assert!(!mesh.has_room(1));
    mesh.push(Quads::at(48));
    assert_eq!(mesh.quads().len(), 2);
    assert!(mesh.truncated);
    // the dropped quad does not widen the bounds
    assert_eq!(mesh.bounds().max[0], 32);
}
