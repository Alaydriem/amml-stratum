// How a ghost mesh job lands in its slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GhostJobMode {
    // The job builds the whole slot mesh.
    Replace,
    // The job adds its mesh to the slot mesh of its group.
    Append,
    // The job adds its mesh to the slot mesh of its group and completes it.
    Finish,
}
