// How a block hides the faces of its neighbours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Occlusion {
    // A full opaque cube: every neighbour face against it is hidden.
    Opaque,
    // A see-through cube (glass, water): it hides only the faces of neighbours in the same group.
    Group(u16),
    // Hides nothing.
    None,
}

impl Occlusion {
    // True when a face of a block with `self` that touches a neighbour with `neighbour` cannot be seen.
    pub fn hidden_by(self, neighbour: Occlusion) -> bool {
        match (self, neighbour) {
            (_, Occlusion::Opaque) => true,
            (Occlusion::Group(own), Occlusion::Group(other)) => own == other,
            _ => false,
        }
    }
}
