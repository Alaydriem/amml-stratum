/// What one mini ghost group upload gave.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GroupFinished {
    pub ok: bool,
    /// The game's tessellator was in use, so nothing was uploaded and the job is not answered.
    pub busy: bool,
    pub vertices: u64,
    pub indices: u64,
}
