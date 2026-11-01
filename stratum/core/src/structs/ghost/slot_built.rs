// The shim's answer for one job it ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SlotBuilt {
    pub slot: u32,
    // The serial of the job this answers.
    pub serial: u32,
    // The generation of the frame whose job was built.
    pub generation: u64,
    // GPU bytes the mesh holds.
    pub bytes: u64,
    pub ok: bool,
}
